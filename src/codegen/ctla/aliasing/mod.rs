use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::codegen::ctla::cfg::CFGFunction;
use crate::codegen::tir::ir::{BlockId, Function, HeapAllocation, TIR, TirBuilder, ValueId};
#[derive(Clone)]
pub struct AliasAndEncapsulationTracker {
    builder: Rc<RefCell<TirBuilder>>,
    pub aliases: HashSet<(u64, String, ValueId)>,
    pub encapsulators: HashSet<(u64, String, ValueId)>,
}
impl AliasAndEncapsulationTracker {
    pub fn new(builder: &Rc<RefCell<TirBuilder>>) -> AliasAndEncapsulationTracker {
        return AliasAndEncapsulationTracker {
            builder: Rc::clone(builder),
            aliases: HashSet::new(),
            encapsulators: HashSet::new(),
        };
    }
    #[allow(unused)]
    pub fn has_alias(&self, original_alloc_id: u64, func_name: &str, alias_id: ValueId) -> bool {
        return self.aliases.get(&(original_alloc_id, func_name.to_string(), alias_id)).is_some();
    }
    #[allow(unused)]
    pub fn has_encapsulator(&self, original_alloc_id: u64, func_name: &str, enc_id: ValueId) -> bool {
        return self.encapsulators.get(&(original_alloc_id, func_name.to_string(), enc_id)).is_some();
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
                let Some(return_alias_param_indexes) = summary_by_func.get(callee_name.as_ref())
                else {
                    return false;
                };

                return_alias_param_indexes.iter().any(|arg_idx| {
                    params.get(*arg_idx).is_some_and(|arg| {
                        AliasAndEncapsulationTracker::value_may_alias_param_with_summaries(
                            func,
                            arg.val,
                            param_value_id,
                            visited,
                            summary_by_func,
                        )
                    })
                })
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
            let summary_snapshot: HashMap<String, Vec<usize>> = cfg_functions
                .iter()
                .map(|cfg_f| {
                    (
                        (*cfg_f.func.name).clone(),
                        cfg_f.returns_alias_of_parameter.clone(),
                    )
                })
                .collect();

            let mut changed = false;
            for cfg_f in cfg_functions.iter_mut() {
                let mut new_summary =
                    AliasAndEncapsulationTracker::find_return_alias_parameter_indexes_with_summaries(
                        &cfg_f.func,
                        &summary_snapshot,
                    );
                new_summary.sort_unstable();
                new_summary.dedup();

                if cfg_f.returns_alias_of_parameter != new_summary {
                    cfg_f.returns_alias_of_parameter = new_summary;
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
    ) {
        loop {
            let mut changed = false;

            let mut new_aliases = alias_values.clone();
            let builder = self.builder.borrow();
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
                                    builder.funcs.iter().find(|f| f.name.as_ref() == callee_name.as_ref())
                                {
                                    for (arg_idx, arg) in params.iter().enumerate() {
                                        if alias_values.contains(&(function_name.clone(), arg.val))
                                            && callee_func.params.get(arg_idx).is_some_and(|callee_param| {
                                                new_aliases.insert(((*callee_func.name).clone(), callee_param.val))
                                            })
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
                            TIR::CallExternFunction(out_id, callee_name, params, _, _, _)
                                if callee_name.as_ref() == "toy_write_to_arr" =>
                            {
                                if params.len() >= 2
                                    && alias_values
                                        .contains(&(function_name.clone(), params[1].val))
                                {
                                    if new_encapsulators
                                        .insert((function_name.clone(), *out_id))
                                    {
                                        changed = true;
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
                                // Propagate encapsulation through call returns when the callee
                                // returns an array value that was written with an aliased value.
                                let callee_returns_encapsulating_array = builder
                                    .funcs
                                    .iter()
                                    .find(|f| f.name.as_ref() == callee_name.as_ref())
                                    .is_some_and(|callee| {
                                        let returned_values: HashSet<ValueId> = callee
                                            .body
                                            .iter()
                                            .filter_map(|b| match b.ins.last() {
                                                Some(TIR::Ret(_, ret_val)) => Some(ret_val.val),
                                                _ => None,
                                            })
                                            .collect();

                                        if returned_values.is_empty() {
                                            return false;
                                        }

                                        callee.body.iter().flat_map(|b| b.ins.iter()).any(|ins| {
                                            matches!(
                                                ins,
                                                TIR::CallExternFunction(_, name, write_params, _, _, _)
                                                    if name.as_ref() == "toy_write_to_arr"
                                                        && write_params.len() >= 2
                                                        && returned_values.contains(&write_params[0].val)
                                                        && alias_values.contains(&((*callee.name).clone(), write_params[1].val))
                                            )
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
        let summary_by_func: HashMap<String, Vec<usize>> = cfg_functions
            .iter()
            .map(|cfg_f| {
                (
                    (*cfg_f.func.name).clone(),
                    cfg_f.returns_alias_of_parameter.clone(),
                )
            })
            .collect();

        let mut value_to_block: HashMap<(String, ValueId), BlockId> = HashMap::new();
        let builder = self.builder.borrow();
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

        self.propagate_aliases(&mut alias_values, summary_by_func, &mut encapsulator_values);
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
