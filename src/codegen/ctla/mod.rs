use super::tir::ir::BlockId;
use crate::{
    codegen::{
        SSAValue,
        ctla::aliasing::AliasAndEncapsulationTracker,
        tir::ir::{Block, Function, HeapAllocation, TIR, TirBuilder, TirType, ValueId},
    },
    driver::Driver,
    errors::ToyError,
};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;
pub mod aliasing;
pub mod cfg;
use cfg::{CFGBlock, CFGFunction, EscapeType};
use serde::{Deserialize, Serialize};
use std::fs;
pub struct CTLA {
    builder: Rc<RefCell<TirBuilder>>,
    cfg_functions: Vec<CFGFunction>,
    alias_detector: AliasAndEncapsulationTracker,
    original_text: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionSummary {
    pub name: String,
    pub aliased_parameters: Vec<usize>,
    pub encapsulated_parameters: Vec<usize>,
    pub escaped_parameters: Vec<usize>,
}
impl FunctionSummary {
    pub fn new(
        name: String,
        aliased_parameters: Vec<usize>,
        encapsulated_parameters: Vec<usize>,
        escaped_parameters: Vec<usize>,
    ) -> FunctionSummary {
        return FunctionSummary {
            name: name,
            aliased_parameters,
            encapsulated_parameters,
            escaped_parameters,
        };
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CTLASchema {
    pub schema_version: u64,
    pub summaries: Vec<FunctionSummary>,
    pub input_hash: String,
    pub module_name: String,
}
impl CTLASchema {
    pub fn new(
        schema_version: u64,
        summaries: Vec<FunctionSummary>,
        input_hash: String,
        module_name: String,
    ) -> CTLASchema {
        return CTLASchema {
            schema_version,
            summaries,
            input_hash,
            module_name,
        };
    }
}
//COMPILE TIME LIFETIME ANALYSIS
impl CTLA {
    pub fn new() -> CTLA {
        let b = Rc::new(RefCell::new(TirBuilder::new()));
        let alias_detector = AliasAndEncapsulationTracker::new(&b);

        CTLA {
            builder: b,
            cfg_functions: vec![],
            alias_detector,
            original_text: None,
        }
    }

    pub fn set_external_modules(&mut self, modules: HashMap<String, Vec<FunctionSummary>>) {
        self.alias_detector.set_external_modules(modules);
    }

    pub fn set_original_text(&mut self, text: String) {
        self.original_text = Some(text);
    }
    pub fn cfg_functions(&self) -> &Vec<CFGFunction> {
        &self.cfg_functions
    }
    #[allow(unused)]
    pub fn alias_tracker(&self) -> &AliasAndEncapsulationTracker {
        &self.alias_detector
    }
    fn is_terminator_ins(&self, ins: &TIR) -> bool {
        return matches!(
            ins,
            TIR::Ret(_, _) | TIR::JumpCond(_, _, _, _) | TIR::JumpBlockUnCond(_, _)
        );
    }
    ///checks whether some SSA value could refer to a specific allocation, but only by following phi chains.
    fn value_may_be_allocation_via_phi(
        &self,
        func: &Function,
        value_id: ValueId,
        alloc: &HeapAllocation,
        visited: &mut HashSet<ValueId>,
    ) -> bool {
        let seeds = HashSet::from([alloc.alloc_ins.val]);
        self.value_may_match_seed_via_phi(func, value_id, &seeds, visited)
    }
    fn value_may_match_seed_via_phi(
        &self,
        func: &Function,
        value_id: ValueId,
        seeds: &HashSet<ValueId>,
        visited: &mut HashSet<ValueId>,
    ) -> bool {
        if seeds.contains(&value_id) {
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
            TIR::Phi(_, block_ids, vals) => block_ids.iter().zip(vals.iter()).any(|(bid, v)| {
                if self
                    .alias_detector
                    .block_has_non_returning_panic_call(func, *bid)
                {
                    return false;
                }
                self.value_may_match_seed_via_phi(func, v.val, seeds, visited)
            }),
            _ => false,
        }
    }
    fn allocation_protected_values_in_function(
        &self,
        alloc: &HeapAllocation,
        function_name: &str,
    ) -> HashSet<ValueId> {
        let mut protected_ids: HashSet<ValueId> = HashSet::new();

        if alloc.function.as_ref() == function_name {
            protected_ids.insert(alloc.alloc_ins.val);
        }

        alloc
            .refs
            .iter()
            .filter(|(f, _, _)| f.as_ref() == function_name)
            .for_each(|(_, _, value_id)| {
                protected_ids.insert(*value_id);
            });

        alloc
            .aliases
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, _, value_id)| {
                protected_ids.insert(*value_id);
            });

        alloc
            .encapsulators
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, _, value_id)| {
                protected_ids.insert(*value_id);
            });

        protected_ids
    }
    fn allocation_tracked_blocks_in_function(
        &self,
        alloc: &HeapAllocation,
        function_name: &str,
    ) -> HashSet<BlockId> {
        let mut tracked_blocks: HashSet<BlockId> = HashSet::new();

        if alloc.function.as_ref() == function_name {
            tracked_blocks.insert(alloc.block);
        }

        alloc
            .refs
            .iter()
            .filter(|(f, _, _)| f.as_ref() == function_name)
            .for_each(|(_, b, _)| {
                tracked_blocks.insert(*b);
            });

        alloc
            .aliases
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, b, _)| {
                tracked_blocks.insert(*b);
            });

        alloc
            .encapsulators
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, b, _)| {
                tracked_blocks.insert(*b);
            });

        tracked_blocks
    }

    /// Finds the exact index in a given block where it is safe to insert a free call
    /// Assumes the block is the correct place for the free
    fn free_insertion_index_for_block(
        &self,
        func: &Function,
        block_id: BlockId,
        alloc: &HeapAllocation,
    ) -> usize {
        let block = func.body.iter().find(|b| b.id == block_id).unwrap();
        let protected_ids = self.allocation_protected_values_in_function(alloc, func.name.as_ref());

        let last_ref_idx = block
            .ins
            .iter()
            .enumerate()
            .filter(|(_, ins)| self.instruction_uses_any_value(ins, &protected_ids))
            .map(|(idx, _)| idx)
            .max();

        let terminator_idx = block
            .ins
            .last()
            .and_then(|ins| self.is_terminator_ins(ins).then_some(block.ins.len() - 1));

        let mut insertion_idx = match last_ref_idx {
            Some(idx) => idx + 1,
            None => terminator_idx.unwrap_or(block.ins.len()),
        };

        let has_same_func_encapsulator =
            alloc.encapsulators.iter().any(|(f, _, _)| *f == *func.name);
        if has_same_func_encapsulator {
            insertion_idx = terminator_idx.unwrap_or(block.ins.len());
        }

        if let Some(term_idx) = terminator_idx {
            if insertion_idx > term_idx {
                insertion_idx = term_idx;
            }
        }

        return insertion_idx;
    }
    fn block_returns_allocation_or_alias(
        &self,
        func: &Function,
        block_id: BlockId,
        alloc: &HeapAllocation,
    ) -> bool {
        let Some(block) = func.body.iter().find(|b| b.id == block_id) else {
            return false;
        };
        let Some(TIR::Ret(_, ret_val)) = block.ins.last() else {
            return false;
        };

        let protected_ids = self.allocation_protected_values_in_function(alloc, func.name.as_ref());
        if protected_ids.contains(&ret_val.val) {
            let ret_is_phi = func
                .body
                .iter()
                .flat_map(|b| b.ins.iter())
                .any(|ins| ins.get_id() == ret_val.val && matches!(ins, TIR::Phi(_, _, _)));
            if !ret_is_phi {
                return true;
            }
        }

        let mut visited = HashSet::new();
        self.value_may_be_allocation_via_phi(func, ret_val.val, alloc, &mut visited)
    }
    /// Determines if the given instruction references any of the value or any of its aliases
    fn instruction_uses_any_value(&self, ins: &TIR, aliases: &HashSet<ValueId>) -> bool {
        let uses = |value: &SSAValue| aliases.contains(&value.val);
        return match ins {
            TIR::ItoF(_, value, _) => uses(value),
            TIR::NumericInfix(_, left, right, _) => uses(left) || uses(right),
            TIR::BoolInfix(_, left, right, _) => uses(left) || uses(right),
            TIR::JumpCond(_, cond, _, _) => uses(cond),
            TIR::Ret(_, value) => uses(value),
            TIR::CallLocalFunction(_, _, params, _, _)
            | TIR::CallExternFunction(_, _, params, _, _, _)
            | TIR::CreateStructLiteral(_, _, params)
            | TIR::Phi(_, _, params) => params.iter().any(uses),
            TIR::ReadStructLiteral(_, struct_value, _) => uses(struct_value),
            TIR::WriteStructLiteral(_, struct_value, _, new_value) => {
                uses(struct_value) || uses(new_value)
            }
            TIR::Not(_, value) => uses(value),
            TIR::IConst(_, _, _)
            | TIR::FConst(_, _, _)
            | TIR::JumpBlockUnCond(_, _)
            | TIR::CreateStructInterface(_, _, _)
            | TIR::GlobalString(_, _) => false,
        };
    }

    /// Determines if a given allocation escapes the function it was created in, escapes the program as a whole, or dies in the function
    fn allocation_escapes(&self, alloc: &HeapAllocation) -> EscapeType {
        let func = {
            let builder = self.builder.borrow();
            builder
                .funcs
                .iter()
                .find(|f| *f.name == *alloc.function)
                .cloned()
                .unwrap()
        };
        let protected_ids = self.allocation_protected_values_in_function(alloc, func.name.as_ref());
        let is_param = func.params.iter().any(|p| p.val == alloc.alloc_ins.val);
        //params always freed by the caller
        if is_param {
            return EscapeType::EscapesFunction;
        }

        //alloc is returned
        for b in &func.body {
            if let Some(TIR::Ret(_, a)) = b.ins.last() {
                if a.val == alloc.alloc_ins.val {
                    return EscapeType::EscapesFunction;
                }
                if protected_ids.contains(&a.val) {
                    let ret_is_phi = func
                        .body
                        .iter()
                        .flat_map(|b| b.ins.iter())
                        .any(|ins| ins.get_id() == a.val && matches!(ins, TIR::Phi(_, _, _)));
                    if !ret_is_phi {
                        return EscapeType::EscapesFunction;
                    }
                }
                let mut visited = HashSet::new();
                if self.value_may_be_allocation_via_phi(&func, a.val, alloc, &mut visited) {
                    return EscapeType::EscapesFunction;
                }
            }
        }

        for b in &func.body {
            for i in &b.ins {
                match i {
                    TIR::CallExternFunction(_, callee_name, p, _, _, doesnt_take_ownership) => {
                        for (idx, arg) in p.iter().enumerate() {
                            let is_alloc_ref = protected_ids.contains(&arg.val) || {
                                let mut visited = HashSet::new();
                                self.value_may_be_allocation_via_phi(
                                    &func,
                                    arg.val,
                                    alloc,
                                    &mut visited,
                                )
                            };

                            if is_alloc_ref {
                                if let Some(summary) = self.alias_detector.get_external_summary(callee_name.as_ref()) {
                                    if summary.escaped_parameters.contains(&idx) {
                                        return EscapeType::EscapesModule;
                                    }
                                } else if !doesnt_take_ownership.get(idx).copied().unwrap_or(false)
                                {
                                    return EscapeType::EscapesProgram;
                                }
                            }
                        }
                    }
                    _ => continue,
                };
            }
        }

        return EscapeType::DoesNotEscape;
    }

    /// determines if he block or any of its children reference the given allocation, has a cycle guard
    fn block_children_reference_allocation(
        &self,
        func: &CFGFunction,
        cfg_b: &CFGBlock,
        alloc: &HeapAllocation,
        tracked_blocks: &HashSet<BlockId>,
        visited: &mut HashSet<BlockId>,
        is_root: bool,
    ) -> bool {
        if visited.contains(&cfg_b.block) {
            return false;
        }
        visited.insert(cfg_b.block);
        if cfg_b.possible_output_blocks.is_empty() {
            return false;
        }

        // only check refs in non-root blocks (successors, not the candidate block itself)
        if !is_root {
            if tracked_blocks.contains(&cfg_b.block) {
                return true;
            }
        }

        for possible_output_block in &cfg_b.possible_output_blocks {
            let child = func
                .cfg_blocks
                .iter()
                .find(|b| b.block == *possible_output_block)
                .unwrap();
            if self.block_children_reference_allocation(
                func,
                child,
                alloc,
                tracked_blocks,
                visited,
                false,
            ) {
                return true;
            }
        }
        false
    }
    /// finds the function and the exact SSAValue where the allocation is initialized and must be freed
    fn find_owning_function(&self, alloc: &HeapAllocation) -> (String, SSAValue) {
        let mut current_func = alloc.function.clone();
        let mut current_val = alloc.alloc_ins.clone();

        loop {
            // find all callers that receive a value from current_func via CallLocalFunction
            // this is ludicrous and needs to be refactored into like 5 separate things.
            let callers: Vec<(Function, Block, SSAValue)> = {
                let builder = self.builder.borrow();
                //also I have seen egyptian hyreoglphyics that make more sense then 4 statements inside 8 lambdas and 2 flat maps.
                builder
                    .funcs
                    .iter()
                    .flat_map(|f| {
                        f.body.iter().flat_map(|b| {
                            b.ins.iter().filter_map(|i| {
                                if let TIR::CallLocalFunction(ret_id, name, _, _, ret_type) = i {
                                    if **name == *current_func {
                                        return Some((
                                            f.clone(),
                                            b.clone(),
                                            SSAValue {
                                                val: *ret_id,
                                                ty: Some(ret_type.clone()),
                                            },
                                        ));
                                    }
                                }
                                if let TIR::CallExternFunction(ret_id, name, _, _, ret_type, _) = i
                                {
                                    if **name == *current_func {
                                        return Some((
                                            f.clone(),
                                            b.clone(),
                                            SSAValue {
                                                val: *ret_id,
                                                ty: Some(ret_type.clone()),
                                            },
                                        ));
                                    }
                                }
                                None
                            })
                        })
                    })
                    .collect()
            };

            if callers.is_empty() {
                // nobody called us, we are the owner
                return ((*current_func).clone(), current_val);
            }

            let mut next_hop: Option<(Box<String>, SSAValue)> = None;
            for (caller_func, caller_block, new_val) in callers {
                // check if THIS function also escapes it
                let test_alloc = HeapAllocation {
                    function: caller_func.name.clone(),
                    alloc_ins: new_val.clone(),
                    block: caller_block.id,
                    refs: alloc
                        .refs
                        .iter()
                        .filter(|(f, _, _)| *f == caller_func.name)
                        .cloned()
                        .collect(),
                    allocation_id: alloc.allocation_id,
                    aliases: BTreeSet::new(),       //temp
                    encapsulators: BTreeSet::new(), //temp
                };
                if self.allocation_escapes(&test_alloc) == EscapeType::DoesNotEscape {
                    return ((*caller_func.name).clone(), new_val);
                }
                if next_hop.is_none() {
                    next_hop = Some((caller_func.name.clone(), new_val));
                }
            }

            if let Some((next_func, next_val)) = next_hop {
                current_func = next_func;
                current_val = next_val;
            } else {
                return ((*current_func).clone(), current_val);
            }
        }
    }
    ///takes an allocation and its owning function and marks the point where the free call should be inserted.
    fn process_non_escaping_allocation(
        &self,
        cfg_func: &CFGFunction,
        func: &Function,
        alloc: &HeapAllocation,
        insertion_points: &mut Vec<(String, BlockId, ValueId, SSAValue, String)>,
    ) {
        if !self.function_has_ssa(func, alloc.alloc_ins.val) {
            return;
        }
        let free_func = self.alloc_type_to_free_func(alloc);
        let origin_block_id = alloc.block;
        let Some(origin_cfg_block) = cfg_func
            .cfg_blocks
            .iter()
            .find(|b| b.block == origin_block_id)
        else {
            return;
        };

        let mut visited: HashSet<BlockId> = HashSet::new();
        let tracked_blocks = self.allocation_tracked_blocks_in_function(alloc, func.name.as_ref());
        let has_child_refs = self.block_children_reference_allocation(
            cfg_func,
            origin_cfg_block,
            alloc,
            &tracked_blocks,
            &mut visited,
            true,
        );

        if !has_child_refs {
            if self.block_returns_allocation_or_alias(func, origin_block_id, alloc) {
                return;
            }
            let insertion_idx = if free_func == "toy_free_arr" {
                func.body
                    .iter()
                    .find(|b| b.id == origin_block_id)
                    .map(|b| b.ins.len().saturating_sub(1))
                    .unwrap()
            } else {
                self.free_insertion_index_for_block(func, origin_block_id, alloc)
            };
            insertion_points.push((
                *func.name.clone(),
                origin_block_id,
                insertion_idx,
                alloc.alloc_ins.clone(),
                free_func,
            ));
            return;
        }

        // Conservative fallback: if value is defined in entry block, free at function return blocks.
        if origin_block_id == func.body[0].id {
            for block in &func.body {
                if matches!(block.ins.last(), Some(TIR::Ret(_, _))) {
                    if self.block_returns_allocation_or_alias(func, block.id, alloc) {
                        continue;
                    }
                    let insertion_idx = if free_func == "toy_free_arr" {
                        block.ins.len().saturating_sub(1)
                    } else {
                        self.free_insertion_index_for_block(func, block.id, alloc)
                    };

                    insertion_points.push((
                        *func.name.clone(),
                        block.id,
                        insertion_idx,
                        alloc.alloc_ins.clone(),
                        free_func.clone(),
                    ));
                }
            }
        }
    }
    /// determines if a given ssa value is in the given function body or parameters
    fn function_has_ssa(&self, func: &Function, value_id: ValueId) -> bool {
        if func.params.iter().any(|p| p.val == value_id) {
            return true;
        }
        func.body
            .iter()
            .any(|b| b.ins.iter().any(|ins| ins.get_id() == value_id))
    }

    /// Returns the instruction at the given function, block, and value, note the inputs are id's NOT indexes
    fn get_alloc_ins(
        &self,
        function_name: &str,
        block_id: BlockId,
        value_id: ValueId,
    ) -> Option<TIR> {
        self.builder
            .borrow()
            .funcs
            .iter()
            .find(|f| *f.name == function_name)
            .and_then(|f| f.body.iter().find(|b| b.id == block_id))
            .and_then(|b| b.ins.iter().find(|ins| ins.get_id() == value_id))
            .cloned()
    }
    /// crude doubleplusungood function that tests if a given function name matches 3 known to return arrays
    fn is_array_allocation_call_name(&self, name: &str) -> bool {
        return name == "toy_malloc_arr" || name == "std::sys::argv" || name == "toy_sys_get_argv";
    }
    /// tries to determine if a function returns an array by name, this is bad however and should in future just ask the TIRBuilder
    fn function_returns_array_allocation(&self, function_name: &str) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == function_name) else {
            return false;
        };

        for block in &func.body {
            if let Some(TIR::Ret(_, ret_ssa)) = block.ins.last() {
                if let Some(ins) = block.ins.iter().find(|i| i.get_id() == ret_ssa.val) {
                    if let TIR::CallExternFunction(_, f_box, _, _, _, _) = ins {
                        if self.is_array_allocation_call_name(f_box) {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }
    /// matches the allocation type to the type of free needed (regular or array), returns that function name
    fn alloc_type_to_free_func(&self, alloc: &HeapAllocation) -> String {
        let alloc_ins = self.get_alloc_ins(&alloc.function, alloc.block, alloc.alloc_ins.val);

        let Some(alloc_ins) = alloc_ins else {
            return "toy_free".to_string();
        };

        match alloc_ins {
            TIR::CallExternFunction(_, f_box, _, _, _, _) => {
                //that argv thing is hacky but I dont know how to say under the hood it calls toy_malloc_arr
                if self.is_array_allocation_call_name(f_box.as_ref()) {
                    return "toy_free_arr".to_string();
                }
                return "toy_free".to_string();
            }
            // For local function calls that return heap-allocated values (strings),
            TIR::CallLocalFunction(_, callee_name, _, _, _) => {
                if self.function_returns_array_allocation(callee_name.as_ref()) {
                    return "toy_free_arr".to_string();
                }
                return "toy_free".to_string();
            }
            _ => unreachable!(),
        };
    }

    /// runs the full pipeline to mark (or intentionally leak) a given allocation
    fn process_allocation(
        &mut self,
        alloc: &mut HeapAllocation,
        insertion_points: &mut Vec<(String, BlockId, ValueId, SSAValue, String)>,
    ) {
        let func = {
            let builder = self.builder.borrow();
            builder
                .funcs
                .iter()
                .find(|f| *f.name == *alloc.function)
                .cloned()
                .unwrap()
        };
        self.alias_detector
            .find_aliases_and_encapsulators(alloc, &mut self.cfg_functions);
        let is_param = func.params.iter().any(|p| p.val == alloc.alloc_ins.val);
        if is_param {
            return;
        }
        let cfg_func = self
            .cfg_functions
            .iter()
            .find(|f| f.func.name == func.name)
            .unwrap();
        let escape_type = self.allocation_escapes(&alloc);
        if escape_type == EscapeType::EscapesProgram || escape_type == EscapeType::EscapesModule {
            //at this pont let it leak, it it escapes the program
            return;
        } else if self.allocation_escapes(&alloc) == EscapeType::DoesNotEscape {
            //if in this branch, the allocation dies in ths function
            self.process_non_escaping_allocation(cfg_func, &func, &alloc, insertion_points);
        } else {
            let (owning_func_name, owning_val) = self.find_owning_function(&alloc);

            let owning_func = {
                let builder = self.builder.borrow();
                builder
                    .funcs
                    .iter()
                    .find(|f| *f.name == owning_func_name)
                    .cloned()
                    .unwrap()
            };

            // find the block containing the call site (where owning_val was defined)
            let owning_block_id = owning_func
                .body
                .iter()
                .find(|b| b.ins.iter().any(|i| i.get_id() == owning_val.val))
                .unwrap()
                .id;

            let owned_alloc = HeapAllocation {
                block: owning_block_id,
                function: Box::new(owning_func_name.clone()),
                alloc_ins: owning_val,
                allocation_id: alloc.allocation_id,
                refs: alloc
                    .refs
                    .iter()
                    .filter(|(f, _, _)| **f == owning_func_name)
                    .cloned()
                    .collect(),
                aliases: alloc
                    .aliases
                    .iter()
                    .filter(|(f, _, _)| *f == owning_func_name)
                    .cloned()
                    .collect(),
                encapsulators: alloc
                    .encapsulators
                    .iter()
                    .filter(|(f, _, _)| *f == owning_func_name)
                    .cloned()
                    .collect(),
            };

            let owning_cfg_func = self
                .cfg_functions
                .iter()
                .find(|f| *f.func.name == owning_func_name)
                .unwrap();

            self.process_non_escaping_allocation(
                owning_cfg_func,
                &owning_func,
                &owned_alloc,
                insertion_points,
            );
        }
    }
    fn populate_parameter_escape_summary(&self, funcs: Vec<CFGFunction>) -> Vec<CFGFunction> {
        let mut new_funcs: Vec<CFGFunction> = vec![];
        for cfg_func in funcs {
            let mut new_cfg = cfg_func.clone();
            for (idx, param) in new_cfg.func.params.iter().enumerate() {
                if param.ty != Some(TirType::Ptr) {
                    continue;
                }

                let seeds = HashSet::from([param.val]);
                let mut param_escapes_program = false;
                for b in &new_cfg.func.body {
                    for ins in &b.ins {
                        if let TIR::CallExternFunction(_, callee, args, _, _, doesnt_take_ownership) = ins {
                            for (arg_idx, arg) in args.iter().enumerate() {
                                let mut visited = HashSet::new();
                                if self.value_may_match_seed_via_phi(
                                    &new_cfg.func,
                                    arg.val,
                                    &seeds,
                                    &mut visited,
                                ) {
                                    if let Some(summary) = self.alias_detector.get_external_summary(callee.as_ref()) {
                                        if summary.escaped_parameters.contains(&arg_idx) {
                                            param_escapes_program = true;
                                        }
                                    } else if !doesnt_take_ownership
                                        .get(arg_idx)
                                        .copied()
                                        .unwrap_or(false)
                                    {
                                        param_escapes_program = true;
                                    }
                                }
                            }
                        }
                    }
                }

                if param_escapes_program {
                    new_cfg.parameter_escapes.push(idx);
                }
            }
            new_cfg.parameter_escapes.sort_unstable();
            new_cfg.parameter_escapes.dedup();
            new_funcs.push(new_cfg);
        }
        return new_funcs;
    }
    /// Runs CTLA Analysis on the given Builder, returns a vec of functions containing the processed code, or an error.
    pub fn analyze(&mut self, builder: TirBuilder) -> Result<Vec<Function>, ToyError> {
        let module_name = Driver::get_current_file_path()
            .and_then(|p| {
                std::path::Path::new(&p)
                    .file_stem()
                    .and_then(|s| s.to_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "module".to_string());
        let external_modules = self.alias_detector.external_modules.clone();
        self.builder = Rc::new(RefCell::new(builder));
        self.alias_detector = AliasAndEncapsulationTracker::new(&self.builder);
        self.alias_detector.set_external_modules(external_modules);
        self.cfg_functions.clear();

        //build per-function CFG graphs
        {
            let mut builder = self.builder.borrow_mut();
            for f in &mut builder.funcs {
                let mut cfg_f = CFGFunction::new(f.to_owned());
                cfg_f.calc_cfg();
                self.cfg_functions.push(cfg_f);
            }
        }
        self.alias_detector
            .populate_return_alias_parameter_summaries(&mut self.cfg_functions);
        self.cfg_functions = self.populate_parameter_escape_summary(self.cfg_functions.clone());
        let mut unique_allocations = self.builder.borrow().detect_unique_heap_allocations();
        let mut insertion_points: Vec<(String, BlockId, ValueId, SSAValue, String)> = vec![];
        for a in &mut unique_allocations {
            self.process_allocation(a, &mut insertion_points);
        }
        // in analyze, before the splice loop
        let dedup_set: HashSet<_> = insertion_points.into_iter().collect();
        insertion_points = dedup_set.into_iter().collect();

        let mut coalesced: HashMap<(String, BlockId, ValueId, String), (usize, SSAValue)> =
            HashMap::new();
        for (name, bid, idx, val, free_name) in insertion_points {
            let key = (name.clone(), bid, val.val, free_name.clone());
            if let Some((existing_idx, _)) = coalesced.get(&key) {
                if idx > *existing_idx {
                    coalesced.insert(key, (idx, val));
                }
            } else {
                coalesced.insert(key, (idx, val));
            }
        }
        insertion_points = coalesced
            .into_iter()
            .map(|((name, bid, _, free_name), (idx, val))| (name, bid, idx, val, free_name))
            .collect();

        let free_sort_rank = |free_name: &str| {
            if free_name == "toy_free_arr" {
                1usize
            } else {
                0usize
            }
        };
        insertion_points.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| free_sort_rank(a.4.as_str()).cmp(&free_sort_rank(b.4.as_str())))
                .then_with(|| a.3.val.cmp(&b.3.val))
        });
        for (name, bid, vid, val, free_name) in insertion_points {
            self.builder
                .borrow_mut()
                .splice_free_before(name, bid, vid, val, free_name);
        }
        let build_dir = Driver::get_build_dir();
        let mut summaries: Vec<FunctionSummary> = vec![];
        for func in &self.cfg_functions {
            summaries.push(FunctionSummary::new(
                *func.func.name.clone(),
                func.returns_alias_of_parameter.clone(),
                func.parameter_encapsulates.clone(),
                func.parameter_escapes.clone(),
            ));
        }

        let mut hasher = ahash::AHasher::default();
        use std::hash::Hasher;
        hasher.write(self.original_text.as_deref().unwrap_or("").as_bytes());
        let hash = format!("{:x}", hasher.finish());


        let schema = CTLASchema::new(1, summaries, hash, module_name.clone());
        let serialized = serde_json::to_string(&schema).unwrap(); //should fix ?

        let _ = fs::create_dir_all(&build_dir);
        let res = fs::write(format!("{}/{}.ctla", build_dir, module_name), serialized);
        match res {
            Err(e) => {
                eprintln!(
                    "[ERROR] Could not write module {} with error {:?}",
                    module_name, e
                )
            }
            _ => {}
        };
        return Ok(self.builder.borrow().funcs.clone());
    }
}

#[cfg(test)]
mod tests;
