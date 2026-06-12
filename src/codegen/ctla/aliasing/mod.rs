use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::codegen::TirType;
use crate::codegen::ctla::FunctionSummary;
use crate::codegen::ctla::cfg::CFGFunction;
use crate::codegen::tir::ir::{BlockId, Function, HeapAllocation, TIR, TirBuilder, ValueId};
#[derive(Clone)]
pub struct AliasAndEncapsulationTracker {
    builder: Rc<RefCell<TirBuilder>>,
    pub aliases: HashSet<(u64, String, ValueId)>,
    pub encapsulators: HashSet<(u64, String, ValueId)>,
    pub external_modules: HashMap<String, Vec<FunctionSummary>>,
    /// Total number of fixed-point iterations across all propagate_aliases calls
    pub total_fp_iters: Cell<u64>,
}
impl AliasAndEncapsulationTracker {
    pub fn new(builder: &Rc<RefCell<TirBuilder>>) -> AliasAndEncapsulationTracker {
        return AliasAndEncapsulationTracker {
            builder: Rc::clone(builder),
            aliases: HashSet::new(),
            encapsulators: HashSet::new(),
            external_modules: HashMap::new(),
            total_fp_iters: Cell::new(0),
        };
    }

    pub fn set_external_modules(&mut self, modules: HashMap<String, Vec<FunctionSummary>>) {
        self.external_modules = modules;
    }

    pub fn get_external_summary(&self, callee_name: &str) -> Option<&FunctionSummary> {
        let parts: Vec<&str> = callee_name.split("::").collect();
        if parts.len() < 2 {
            return None;
        }
        let module_name = parts[..parts.len() - 1].join(".");
        if let Some(summaries) = self.external_modules.get(&module_name) {
            return summaries.iter().find(|s| s.name == callee_name);
        }
        // Struct methods embed the struct name (e.g. "std::time::Date:::to_str_struct")
        // which produces module key "std.time.Date" instead of "std.time". Try shorter prefixes.
        for end in (2..parts.len()).rev() {
            let shorter = parts[..end].join(".");
            if let Some(summaries) = self.external_modules.get(&shorter) {
                if let Some(s) = summaries.iter().find(|s| s.name == callee_name) {
                    return Some(s);
                }
            }
        }
        None
    }

    /// Owned (heap) fields of a struct value (e.g. a local `toy_malloc_struct` result's struct
    /// literal arg). Mirrors `populate_return_owned_fields` but for an arbitrary struct value, so
    /// local structs get the same field-deep-free treatment as returned ones.
    pub fn owned_fields_of_struct_value(
        &self,
        func: &Function,
        struct_value: ValueId,
        cfg_functions: &[CFGFunction],
    ) -> Vec<super::OwnedField> {
        let mut summary_by_func: HashMap<String, Vec<usize>> = cfg_functions
            .iter()
            .map(|cfg_f| {
                (
                    (*cfg_f.func.name).clone(),
                    cfg_f.returns_alias_of_parameter.clone(),
                )
            })
            .collect();
        let mut owned_fields_by_func: HashMap<String, Vec<super::OwnedField>> = cfg_functions
            .iter()
            .map(|cfg_f| ((*cfg_f.func.name).clone(), cfg_f.return_owned_fields.clone()))
            .collect();
        for summaries in self.external_modules.values() {
            for summary in summaries {
                summary_by_func
                    .entry(summary.name.clone())
                    .or_insert_with(|| summary.aliased_parameters.clone());
                owned_fields_by_func
                    .insert(summary.name.clone(), summary.return_owned_fields.clone());
            }
        }
        let mut visited = HashSet::new();
        let mut fields = Self::collect_owned_fields(
            func,
            struct_value,
            &mut visited,
            &summary_by_func,
            &owned_fields_by_func,
        );
        fields.sort_by_key(|f| f.index);
        fields.dedup();
        fields
    }

    #[allow(unused)]
    pub fn has_alias(&self, original_alloc_id: u64, func_name: &str, alias_id: ValueId) -> bool {
        return self
            .aliases
            .get(&(original_alloc_id, func_name.to_string(), alias_id))
            .is_some();
    }
    #[allow(unused)]
    pub fn has_encapsulator(
        &self,
        original_alloc_id: u64,
        func_name: &str,
        enc_id: ValueId,
    ) -> bool {
        return self
            .encapsulators
            .get(&(original_alloc_id, func_name.to_string(), enc_id))
            .is_some();
    }
    ///just tests if the block contains a call to panic
    pub fn block_has_non_returning_panic_call(&self, func: &Function, block_id: BlockId) -> bool {
        let Some(block) = func.body.iter().find(|b| b.id == block_id) else {
            return false;
        };
        return block.ins.iter().any(|ins| {
            matches!(
                ins,
                TIR::CallExternFunction(_, name, _, _, _, _) if **name == *"std::sys::panic_str"
            )
        });
    }
    /// checks whether a value may alias a given parameter by walking phi nodes and call-return summaries
    fn value_may_alias_param_with_summaries(
        func: &Function,
        value_id: ValueId,
        param_value_id: ValueId,
        visited: &mut HashSet<ValueId>,
        summary_by_func: &HashMap<String, Vec<usize>>,
    ) -> bool {
        if value_id == param_value_id {
            return true;
        }
        if visited.contains(&value_id) {
            return false;
        }
        visited.insert(value_id);

        let maybe_ins = func
            .body
            .iter()
            .flat_map(|b| b.ins.iter())
            .find(|ins| ins.get_id() == value_id);

        let Some(ins) = maybe_ins else {
            return false;
        };

        match ins {
            TIR::Phi(_, _, vals) => vals.iter().any(|v| {
                AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                    func,
                    v.val,
                    param_value_id,
                    visited,
                    summary_by_func,
                )
            }),
            TIR::CallLocalFunction(_, callee_name, params, _, _)
            | TIR::CallExternFunction(_, callee_name, params, _, _, _) => {
                if let Some(return_alias_param_indexes) = summary_by_func.get(callee_name.as_ref())
                {
                    return return_alias_param_indexes.iter().any(|arg_idx| {
                        params.get(*arg_idx).is_some_and(|arg| {
                            AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                                func,
                                arg.val,
                                param_value_id,
                                visited,
                                summary_by_func,
                            )
                        })
                    });
                }

                // No summary available. For non-allocator extern functions,
                // conservatively assume the return may alias any pointer argument.
                let is_allocator = matches!(ins, TIR::CallExternFunction(_, _, _, true, _, _));
                if !is_allocator {
                    params.iter().any(|arg| {
                        AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                            func,
                            arg.val,
                            param_value_id,
                            visited,
                            summary_by_func,
                        )
                    })
                } else {
                    false
                }
            }
            TIR::CreateStructLiteral(_, _, fields) => fields.iter().any(|field| {
                AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                    func,
                    field.val,
                    param_value_id,
                    visited,
                    summary_by_func,
                )
            }),
            TIR::WriteStructLiteral(_, base_struct, _, new_val) => {
                AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                    func,
                    base_struct.val,
                    param_value_id,
                    visited,
                    summary_by_func,
                ) || AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                    func,
                    new_val.val,
                    param_value_id,
                    visited,
                    summary_by_func,
                )
            }
            _ => false,
        }
    }
    /// computes which parameter indexes in this function may flow to a return value
    fn find_return_alias_parameter_indexes_with_summaries(
        func: &Function,
        summary_by_func: &HashMap<String, Vec<usize>>,
    ) -> Vec<usize> {
        let return_values: Vec<ValueId> = func
            .body
            .iter()
            .filter_map(|b| match b.ins.last() {
                Some(TIR::Ret(_, ret_val)) => Some(ret_val.val),
                _ => None,
            })
            .collect();

        let mut alias_param_indexes = vec![];
        for (idx, param) in func.params.iter().enumerate() {
            let param_is_returned_or_aliased = return_values.iter().any(|ret_val| {
                let mut visited = HashSet::new();
                AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                    func,
                    *ret_val,
                    param.val,
                    &mut visited,
                    summary_by_func,
                )
            });
            if param_is_returned_or_aliased {
                alias_param_indexes.push(idx);
            }
        }

        return alias_param_indexes;
    }
    /// repeatedly recomputes return alias summaries for all cfg functions until a fixed point is reached
    /// Will determine if any of the possible return values are aliases of any of the parameters
    pub fn populate_return_alias_parameter_summaries(&self, cfg_functions: &mut [CFGFunction]) {
        loop {
            let mut summary_snapshot: HashMap<String, Vec<usize>> = cfg_functions
                .iter()
                .map(|cfg_f| {
                    (
                        (*cfg_f.func.name).clone(),
                        cfg_f.returns_alias_of_parameter.clone(),
                    )
                })
                .collect();

            for summaries in self.external_modules.values() {
                for summary in summaries {
                    summary_snapshot
                        .insert(summary.name.clone(), summary.aliased_parameters.clone());
                }
            }

            let mut changed = false;
            for cfg_f in cfg_functions.iter_mut() {
                let mut new_summary =
                    AliasAndEncapsulationTracker::find_return_alias_parameter_indexes_with_summaries(
                        &cfg_f.func,
                        &summary_snapshot,
                    );
                new_summary.sort_unstable();
                new_summary.dedup();

                if cfg_f.returns_alias_of_parameter != new_summary
                    || cfg_f.parameter_encapsulates != new_summary
                {
                    cfg_f.returns_alias_of_parameter = new_summary;
                    cfg_f.parameter_encapsulates = cfg_f.returns_alias_of_parameter.clone();
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }
    }
        fn value_is_array(func: &Function, value_id: ValueId, visited: &mut HashSet<ValueId>) -> bool {
            if visited.contains(&value_id) {
                return false;
            }
            visited.insert(value_id);
            let maybe_ins = func.body.iter().flat_map(|b| b.ins.iter()).find(|ins| ins.get_id() == value_id);
            match maybe_ins {
                Some(TIR::CallExternFunction(_, name, _, _, _, _)) => {
                    name.as_ref() == "toy_read_from_arr" || name.as_ref() == "toy_malloc_arr"
                }
                Some(TIR::Phi(_, _, vals)) => {
                    vals.iter().any(|v| Self::value_is_array(func, v.val, visited))
                }
                Some(TIR::CallLocalFunction(_, _, _, true, _)) => {
                    // Conservative: local function returning an allocator could be array or string.
                    // Check if its return type looks array-ish by looking at what flows into it.
                    false
                }
                _ => false,
            }
        }

        fn collect_owned_fields(
        func: &Function,
        value_id: ValueId,
        visited: &mut HashSet<ValueId>,
        summary_by_func: &HashMap<String, Vec<usize>>,
        owned_fields_by_func: &HashMap<String, Vec<super::OwnedField>>
    ) -> Vec<super::OwnedField> {
        if visited.contains(&value_id) {
            return vec![];
        }
        visited.insert(value_id);

        let maybe_ins = func
            .body
            .iter()
            .flat_map(|b| b.ins.iter())
            .find(|ins| ins.get_id() == value_id);

        let Some(ins) = maybe_ins else {
            return vec![];
        };

        match ins {
            TIR::Phi(_, _, vals) => {
                let mut res = vec![];
                for v in vals {
                    res.extend(AliasAndEncapsulationTracker::collect_owned_fields(
                        func, v.val, visited, summary_by_func, owned_fields_by_func,
                    ));
                }
                res
            }
            TIR::CallLocalFunction(_, callee_name, args, _, _)
            | TIR::CallExternFunction(_, callee_name, args, _, _, _) => {
                // If the callee has known owned fields, use those
                if let Some(fields) = owned_fields_by_func.get(callee_name.as_ref()) {
                    if !fields.is_empty() {
                        return fields.clone();
                    }
                }
                // If the return aliases a parameter, follow through to that argument
                if let Some(alias_params) = summary_by_func.get(callee_name.as_ref()) {
                    let mut res = vec![];
                    for &param_idx in alias_params {
                        if let Some(arg) = args.get(param_idx) {
                            res.extend(AliasAndEncapsulationTracker::collect_owned_fields(
                                func, arg.val, visited, summary_by_func, owned_fields_by_func,
                            ));
                        }
                    }
                    res
                } else {
                    vec![]
                }
            }
            TIR::CreateStructLiteral(_, ty, args) => {
                let mut res = vec![];
                if let TirType::StructInterface(types) = ty {
                    for (i, (arg, field_ty)) in args.iter().zip(types.iter()).enumerate() {
                        if matches!(field_ty, TirType::Ptr | TirType::StructInterface(_)) {
                            let mut aliases_param = false;
                            for param in &func.params {
                                let mut v2 = HashSet::new();
                                if AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                                    func,
                                    arg.val,
                                    param.val,
                                    &mut v2,
                                    summary_by_func,
                                ) {
                                    aliases_param = true;
                                    break;
                                }
                            }
                            if !aliases_param {
                                let mut arr_visited = HashSet::new();
                                let is_array = Self::value_is_array(func, arg.val, &mut arr_visited);
                                res.push(super::OwnedField { index: i, is_array });
                            }
                        }
                    }
                }
                res
            }
            _ => vec![],
        }
    }
    pub fn populate_return_owned_fields(&self, cfg_functions: &mut [CFGFunction]) {
        let mut summary_by_func: HashMap<String, Vec<usize>> = cfg_functions
            .iter()
            .map(|cfg_f| {
                (
                    (*cfg_f.func.name).clone(),
                    cfg_f.returns_alias_of_parameter.clone(),
                )
            })
            .collect();
        for summaries in self.external_modules.values() {
            for summary in summaries {
                summary_by_func
                    .entry(summary.name.clone())
                    .or_insert_with(|| summary.aliased_parameters.clone());
            }
        }
            
        loop {
            let mut owned_fields_snapshot: HashMap<String, Vec<super::OwnedField>> = cfg_functions
                .iter()
                .map(|cfg_f| {
                    (
                        (*cfg_f.func.name).clone(),
                        cfg_f.return_owned_fields.clone(),
                    )
                })
                .collect();

            for summaries in self.external_modules.values() {
                for summary in summaries {
                    owned_fields_snapshot
                        .insert(summary.name.clone(), summary.return_owned_fields.clone());
                }
            }

            let mut changed = false;
            for cfg_f in cfg_functions.iter_mut() {
                let mut new_owned_fields = vec![];

                let return_values: Vec<ValueId> = cfg_f.func
                    .body
                    .iter()
                    .filter_map(|b| match b.ins.last() {
                        Some(TIR::Ret(_, ret_val)) => Some(ret_val.val),
                        _ => None,
                    })
                    .collect();

                for ret_val in return_values {
                    let mut visited = HashSet::new();
                    new_owned_fields.extend(AliasAndEncapsulationTracker::collect_owned_fields(
                        &cfg_f.func,
                        ret_val,
                        &mut visited,
                        &summary_by_func,
                        &owned_fields_snapshot,
                    ));
                }

                new_owned_fields.sort_by_key(|f| f.index);
                new_owned_fields.dedup();

                if cfg_f.return_owned_fields != new_owned_fields {
                    cfg_f.return_owned_fields = new_owned_fields;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }
    fn propagate_aliases(
        &self,
        alias_values: &mut HashSet<(String, ValueId)>,
        summary_by_func: HashMap<String, Vec<usize>>,
        encapsulator_values: &mut HashSet<(String, ValueId)>,
        encapsulates_pairs_by_func: &HashMap<String, Vec<(usize, usize)>>,
    ) {
        let builder = self.builder.borrow();

        // O(1) function lookup instead of linear scan on every call instruction.
        let func_by_name: HashMap<&str, &Function> = builder
            .funcs
            .iter()
            .map(|f| (f.name.as_ref().as_str(), f))
            .collect();

        // Precompute the static part of the callee_returns_encapsulating_array check.
        // For each function, store the set of value IDs that are written to a returned
        // array via toy_write_to_arr. Only the alias_values.contains() part is dynamic,
        // so this avoids re-scanning all callee instructions on every fixed-point iteration.
        let mut arr_write_vals_by_callee: HashMap<String, HashSet<ValueId>> = HashMap::new();
        for func in &builder.funcs {
            let returned_values: HashSet<ValueId> = func
                .body
                .iter()
                .filter_map(|b| match b.ins.last() {
                    Some(TIR::Ret(_, ret_val)) => Some(ret_val.val),
                    _ => None,
                })
                .collect();
            if returned_values.is_empty() {
                continue;
            }
            let write_vals: HashSet<ValueId> = func
                .body
                .iter()
                .flat_map(|b| b.ins.iter())
                .filter_map(|ins| match ins {
                    TIR::CallExternFunction(_, name, wp, _, _, _)
                        if matches!(
                            name.as_str(),
                            "toy_write_to_arr"
                                | "toy_write_to_arr_borrowed"
                                | "toy_arr_swap"
                                | "toy_arr_swap_borrowed"
                        ) && wp.len() >= 2
                            && returned_values.contains(&wp[0].val) =>
                    {
                        Some(wp[1].val)
                    }
                    _ => None,
                })
                .collect();
            if !write_vals.is_empty() {
                arr_write_vals_by_callee.insert((*func.name).clone(), write_vals);
            }
        }


        loop {
            self.total_fp_iters.set(self.total_fp_iters.get() + 1);
            let mut changed = false;

            let mut new_aliases = alias_values.clone();
            for f in &builder.funcs {
                let function_name = (*f.name).clone();
                for block in &f.body {
                    for ins in &block.ins {
                        match ins {
                            TIR::Phi(out_id, block_ids, vals) => {
                                if block_ids.iter().zip(vals.iter()).any(|(bid, v)| {
                                    if self.block_has_non_returning_panic_call(f, *bid) {
                                        return false;
                                    }
                                    alias_values.contains(&(function_name.clone(), v.val))
                                }) {
                                    if new_aliases.insert((function_name.clone(), *out_id)) {
                                        changed = true;
                                    }
                                }
                            }
                            TIR::CallLocalFunction(out_id, callee_name, params, _, _) => {
                                if let Some(callee_func) =
                                    func_by_name.get(callee_name.as_ref().as_str())
                                {
                                    for (arg_idx, arg) in params.iter().enumerate() {
                                        if alias_values.contains(&(function_name.clone(), arg.val))
                                            && callee_func.params.get(arg_idx).is_some_and(
                                                |callee_param| {
                                                    let did_insert = new_aliases.insert((
                                                        (*callee_func.name).clone(),
                                                        callee_param.val,
                                                    ));
                                                    did_insert
                                                },
                                            )
                                        {
                                            changed = true;
                                        }
                                    }
                                }

                                let Some(return_alias_param_indexes) =
                                    summary_by_func.get(callee_name.as_ref())
                                else {
                                    continue;
                                };

                                if return_alias_param_indexes.iter().any(|arg_idx| {
                                    params.get(*arg_idx).is_some_and(|arg| {
                                        alias_values.contains(&(function_name.clone(), arg.val))
                                    })
                                }) {
                                    if new_aliases.insert((function_name.clone(), *out_id)) {
                                        changed = true;
                                    }
                                }
                            }
                            TIR::CallExternFunction(out_id, callee_name, params, _, _, _) => {
                                let Some(return_alias_param_indexes) =
                                    summary_by_func.get(callee_name.as_ref())
                                else {
                                    continue;
                                };

                                if return_alias_param_indexes.iter().any(|arg_idx| {
                                    params.get(*arg_idx).is_some_and(|arg| {
                                        alias_values.contains(&(function_name.clone(), arg.val))
                                    })
                                }) {
                                    if new_aliases.insert((function_name.clone(), *out_id)) {
                                        changed = true;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            *alias_values = new_aliases;

            let mut new_encapsulators = encapsulator_values.clone();
            for f in &builder.funcs {
                let function_name = (*f.name).clone();
                for block in &f.body {
                    for ins in &block.ins {
                        match ins {
                            TIR::CreateStructLiteral(out_id, _, params) => {
                                if params.iter().any(|param| {
                                    alias_values.contains(&(function_name.clone(), param.val))
                                }) {
                                    if new_encapsulators.insert((function_name.clone(), *out_id)) {
                                        changed = true;
                                    }
                                }
                            }
                            TIR::WriteStructLiteral(_, struct_value, _, new_value) => {
                                if alias_values.contains(&(function_name.clone(), new_value.val)) {
                                    if new_encapsulators
                                        .insert((function_name.clone(), struct_value.val))
                                    {
                                        changed = true;
                                    }
                                }
                            }
                            TIR::CallLocalFunction(_, callee_name, caller_args, _, _)
                            | TIR::CallExternFunction(_, callee_name, caller_args, _, _, _) => {
                                if let Some(pairs) = encapsulates_pairs_by_func.get(callee_name.as_ref()) {
                                    for &(arr_param_idx, elem_param_idx) in pairs {
                                        if let (Some(arr_arg), Some(elem_arg)) = (caller_args.get(arr_param_idx), caller_args.get(elem_param_idx)) {
                                            if alias_values.contains(&(function_name.clone(), elem_arg.val))
                                                || encapsulator_values.contains(&(function_name.clone(), elem_arg.val))
                                            {
                                                if new_encapsulators.insert((function_name.clone(), arr_arg.val)) {
                                                    changed = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            let mut enc_alias_closure = new_encapsulators.clone();
            for f in &builder.funcs {
                let function_name = (*f.name).clone();
                for block in &f.body {
                    for ins in &block.ins {
                        match ins {
                            TIR::Phi(out_id, block_ids, vals) => {
                                if block_ids.iter().zip(vals.iter()).any(|(bid, v)| {
                                    if self.block_has_non_returning_panic_call(f, *bid) {
                                        return false;
                                    }
                                    new_encapsulators.contains(&(function_name.clone(), v.val))
                                }) {
                                    if enc_alias_closure.insert((function_name.clone(), *out_id)) {
                                        changed = true;
                                    }
                                }
                            }
                            TIR::CallLocalFunction(out_id, callee_name, params, _, _)
                            | TIR::CallExternFunction(out_id, callee_name, params, _, _, _) => {
                                // Use precomputed map: O(precomputed_pairs) instead of
                                // O(functions × instructions) per call per iteration.
                                let callee_returns_encapsulating_array = arr_write_vals_by_callee
                                    .get(callee_name.as_ref())
                                    .map_or(false, |write_vals| {
                                        write_vals.iter().any(|v| {
                                            alias_values.contains(&((**callee_name).clone(), *v))
                                        })
                                    });
                                if callee_returns_encapsulating_array
                                    && enc_alias_closure.insert((function_name.clone(), *out_id))
                                {
                                    changed = true;
                                }

                                let Some(return_alias_param_indexes) =
                                    summary_by_func.get(callee_name.as_ref())
                                else {
                                    continue;
                                };

                                if return_alias_param_indexes.iter().any(|arg_idx| {
                                    params.get(*arg_idx).is_some_and(|arg| {
                                        new_encapsulators
                                            .contains(&(function_name.clone(), arg.val))
                                    })
                                }) {
                                    if enc_alias_closure.insert((function_name.clone(), *out_id)) {
                                        changed = true;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            *encapsulator_values = enc_alias_closure;

            if !changed {
                break;
            }
        }
    }
    /// finds all aliases and encapsulators for an allocation, including transitive alias<->encapsulator closure
    pub fn find_aliases_and_encapsulators(
        &mut self,
        alloc: &mut HeapAllocation,
        cfg_functions: &mut Vec<CFGFunction>,
    ) {
        let mut summary_by_func: HashMap<String, Vec<usize>> = cfg_functions
            .iter()
            .map(|cfg_f| {
                (
                    (*cfg_f.func.name).clone(),
                    cfg_f.returns_alias_of_parameter.clone(),
                )
            })
            .collect();

        for summaries in self.external_modules.values() {
            for summary in summaries {
                summary_by_func.insert(summary.name.clone(), summary.aliased_parameters.clone());
            }
        }

        // Build encapsulates_pairs_by_func: functions that store param[elem] into param[arr].
        // toy_write_to_arr has this built-in (arr=0, elem=1).
        // Local functions compute it from TIR; external functions from their CTLA summary.
        let mut encapsulates_pairs_by_func: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        encapsulates_pairs_by_func.insert("toy_write_to_arr".to_string(), vec![(0, 1)]);
        encapsulates_pairs_by_func.insert("toy_write_to_arr_borrowed".to_string(), vec![(0, 1)]);
        encapsulates_pairs_by_func.insert("toy_arr_swap".to_string(), vec![(0, 1)]);
        encapsulates_pairs_by_func.insert("toy_arr_swap_borrowed".to_string(), vec![(0, 1)]);
        for cfg_f in cfg_functions.iter() {
            if !cfg_f.param_encapsulates_pairs.is_empty() {
                encapsulates_pairs_by_func.insert((*cfg_f.func.name).clone(), cfg_f.param_encapsulates_pairs.clone());
            }
        }
        for summaries in self.external_modules.values() {
            for summary in summaries {
                if !summary.param_encapsulates_pairs.is_empty() {
                    encapsulates_pairs_by_func.insert(summary.name.clone(), summary.param_encapsulates_pairs.clone());
                }
            }
        }

        let builder = self.builder.borrow();
        let capacity = builder
            .funcs
            .iter()
            .flat_map(|f| &f.body)
            .map(|block| block.ins.len())
            .sum();

        let mut value_to_block: HashMap<(String, ValueId), BlockId> =
            HashMap::with_capacity(capacity);
        for f in &builder.funcs {
            let function_name = (*f.name).clone();
            for block in &f.body {
                for ins in &block.ins {
                    value_to_block.insert((function_name.clone(), ins.get_id()), block.id);
                }
            }
        }

        let mut alias_values: HashSet<(String, ValueId)> = HashSet::new();
        alias_values.insert(((*alloc.function).clone(), alloc.alloc_ins.val));
        for (function_name, _, value_id) in &alloc.refs {
            alias_values.insert((function_name.as_ref().clone(), *value_id));
        }

        let mut encapsulator_values: HashSet<(String, ValueId)> = HashSet::new();

        self.propagate_aliases(&mut alias_values, summary_by_func, &mut encapsulator_values, &encapsulates_pairs_by_func);
        let alloc_key = alloc.alloc_ins.val as u64;
        self.aliases.extend(
            alias_values
                .iter()
                .map(|(function_name, value_id)| (alloc_key, function_name.clone(), *value_id)),
        );
        self.encapsulators.extend(
            encapsulator_values
                .iter()
                .map(|(function_name, value_id)| (alloc_key, function_name.clone(), *value_id)),
        );
        alloc.aliases.clear();
        for (function_name, value_id) in &alias_values {
            if function_name == alloc.function.as_ref() && *value_id == alloc.alloc_ins.val {
                continue;
            }

            let block_id = value_to_block
                .get(&(function_name.clone(), *value_id))
                .copied()
                .or_else(|| {
                    alloc.refs.iter().find_map(|(f, b, v)| {
                        if f.as_ref() == function_name && *v == *value_id {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                });

            if let Some(block_id) = block_id {
                alloc
                    .aliases
                    .insert((function_name.clone(), block_id, *value_id));
            }
        }

        alloc.encapsulators.clear();
        for (function_name, value_id) in &encapsulator_values {
            let block_id = value_to_block
                .get(&(function_name.clone(), *value_id))
                .copied()
                .or_else(|| {
                    alloc.refs.iter().find_map(|(f, b, v)| {
                        if f.as_ref() == function_name && *v == *value_id {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                })
                .or_else(|| {
                    // Parameters have no instruction entry in value_to_block; use the entry block.
                    builder.funcs.iter()
                        .find(|f| *f.name == *function_name)
                        .and_then(|f| {
                            if f.params.iter().any(|p| p.val == *value_id) {
                                f.body.first().map(|b| b.id)
                            } else {
                                None
                            }
                        })
                });

            if let Some(block_id) = block_id {
                alloc
                    .encapsulators
                    .insert((function_name.clone(), block_id, *value_id));
            }
        }
    }
}

#[cfg(test)]
mod tests;
