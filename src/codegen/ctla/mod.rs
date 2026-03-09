use crate::{
    codegen::{
        SSAValue,
        tir::ir::{Block, Function, HeapAllocation, TIR, TirBuilder, ValueId},
    },
    errors::ToyError,
};
use std::collections::{BTreeSet, HashMap, HashSet};

use super::tir::ir::BlockId;

pub struct CTLA {
    builder: TirBuilder,
    cfg_functions: Vec<CFGFunction>,
}
#[derive(Debug, Clone, PartialEq)]
enum EscapeType {
    EscapesProgram,
    EscapesFunction,
    DoesNotEscape,
}
#[derive(Debug, Clone)]
///will ALWAYS share a block id with its internal block
struct CFGBlock {
    block: BlockId,
    ///id of all the blocks that could cause the block to run
    possible_input_blocks: Vec<BlockId>,

    ///id of all the possible blocks it could output to
    possible_output_blocks: Vec<BlockId>,
}
impl CFGBlock {
    pub fn new(id: BlockId) -> CFGBlock {
        return CFGBlock {
            block: id,
            possible_input_blocks: vec![],
            possible_output_blocks: vec![],
        };
    }
}
struct CFGFunction {
    func: Function,
    /// parameter indexes whose value may be returned (possibly via phi chains)
    returns_alias_of_parameter: Vec<usize>,
    ///block id -> index in funcs.block
    block_id_to_index: HashMap<BlockId, usize>,
    cfg_blocks: Vec<CFGBlock>,
    ///maps a block id to the id's of all the different blocks that could input to it
    block_id_to_inputs: HashMap<BlockId, Vec<BlockId>>,
    visited_blocks: HashSet<BlockId>,
}
impl CFGFunction {
    pub fn new(func: Function) -> CFGFunction {
        let mut id_to_idx = HashMap::new();
        for (i, b) in func.body.iter().enumerate() {
            id_to_idx.insert(b.id, i);
        }

        return CFGFunction {
            func,
            returns_alias_of_parameter: vec![],
            block_id_to_index: id_to_idx,
            cfg_blocks: vec![],
            block_id_to_inputs: HashMap::new(),
            visited_blocks: HashSet::new(),
        };
    }

    /// Calculates the control flow graph for a given block in the CFGFunction
    /// Will panic if the blockID does not exist, or if the last element is not a TIR::Ret, JumpCond, or JumpUnCond
    /// Will build the CFG for the block and all of its children, DCE happens incidentally, but the code is not eliminated until LLVM
    fn calc_block_cfg(&mut self, id: BlockId) {
        if self.visited_blocks.contains(&id) {
            return;
        }
        //start with entry block, which must always be func.body[0]
        let block = self.func.body[self.block_id_to_index.get(&id).unwrap().to_owned()].clone();
        let mut block_cfg = CFGBlock::new(block.id);
        let start_final_ins = block.ins.last().unwrap();
        match start_final_ins {
            &TIR::Ret(_, _) => {
                self.cfg_blocks.push(block_cfg);
                self.visited_blocks.insert(id);
                return; //leaf node
            }
            &TIR::JumpCond(_, _, b1id, b2id) => {
                //make sure b1 and b2 know this block input to it
                block_cfg.possible_output_blocks = vec![b1id, b2id];
                if self.block_id_to_inputs.contains_key(&b1id) {
                    let mut v = self.block_id_to_inputs.get(&b1id).unwrap().to_owned();
                    v.push(id);
                    self.block_id_to_inputs.insert(b1id, v);
                } else {
                    self.block_id_to_inputs.insert(b1id, vec![id]);
                }
                if self.block_id_to_inputs.contains_key(&b2id) {
                    let mut v = self.block_id_to_inputs.get(&b2id).unwrap().to_owned();
                    v.push(id);
                    self.block_id_to_inputs.insert(b2id, v);
                } else {
                    self.block_id_to_inputs.insert(b2id, vec![id]);
                }
                self.cfg_blocks.push(block_cfg);
                self.visited_blocks.insert(id);
                self.calc_block_cfg(b1id);
                self.calc_block_cfg(b2id);
            }
            &TIR::JumpBlockUnCond(_, bid) => {
                block_cfg.possible_output_blocks = vec![bid];
                if self.block_id_to_inputs.contains_key(&bid) {
                    let mut v = self.block_id_to_inputs.get(&bid).unwrap().to_owned();
                    v.push(id);
                    self.block_id_to_inputs.insert(bid, v);
                } else {
                    self.block_id_to_inputs.insert(bid, vec![id]);
                }
                self.cfg_blocks.push(block_cfg);
                self.visited_blocks.insert(id);
                self.calc_block_cfg(bid);
            }
            _ => unreachable!(),
        };
    }

    /// A wrapper for calc_block_cfg that starts it at the root node, and then retroactively updates the inputs after the output CFG has been calculated
    fn calc_cfg(&mut self) {
        //build base tree - DCE covered by LLVM, they will do it better than I can anyways

        //no inputs for  start block
        self.block_id_to_inputs.insert(self.func.body[0].id, vec![]);
        self.calc_block_cfg(self.func.body[0].id);
        for b in &mut self.cfg_blocks {
            b.possible_input_blocks = self.block_id_to_inputs.get(&b.block).unwrap().to_owned();
        }
    }
}
//COMPILE TIME LIFETIME ANALYSIS
impl CTLA {
    pub fn new() -> CTLA {
        return CTLA {
            builder: TirBuilder::new(),
            cfg_functions: vec![],
        };
    }
    fn is_terminator_ins(&self, ins: &TIR) -> bool {
        return matches!(
            ins,
            TIR::Ret(_, _) | TIR::JumpCond(_, _, _, _) | TIR::JumpBlockUnCond(_, _)
        );
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
                CTLA::value_may_alias_param_with_summaries(
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
                        CTLA::value_may_alias_param_with_summaries(
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
                CTLA::value_may_alias_param_with_summaries(
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

        alias_param_indexes
    }
    /// repeatedly recomputes return alias summaries for all cfg functions until a fixed point is reached
    fn populate_return_alias_parameter_summaries(&mut self) {
        loop {
            let summary_snapshot: HashMap<String, Vec<usize>> = self
                .cfg_functions
                .iter()
                .map(|cfg_f| {
                    (
                        (*cfg_f.func.name).clone(),
                        cfg_f.returns_alias_of_parameter.clone(),
                    )
                })
                .collect();

            let mut changed = false;
            for cfg_f in &mut self.cfg_functions {
                let mut new_summary = CTLA::find_return_alias_parameter_indexes_with_summaries(
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
                if self.block_has_non_returning_panic_call(func, *bid) {
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
    ///just tests if the block contains a call to panic
    fn block_has_non_returning_panic_call(&self, func: &Function, block_id: BlockId) -> bool {
        let Some(block) = func.body.iter().find(|b| b.id == block_id) else {
            return false;
        };
        block.ins.iter().any(|ins| {
            matches!(
                ins,
                TIR::CallExternFunction(_, name, _, _, _, _) if **name == *"std::sys::panic_str"
            )
        })
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

        let has_same_func_encapsulator = alloc
            .encapsulators
            .iter()
            .any(|(f, _, _)| *f == *func.name);
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
        let func = self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == *alloc.function)
            .unwrap();
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
                if self.value_may_be_allocation_via_phi(func, a.val, alloc, &mut visited) {
                    return EscapeType::EscapesFunction;
                }
            }
        }

        for b in &func.body {
            for i in &b.ins {
                match i {
                    //only extern func calls are considered escapes because in regular func calls, everything s passed by reference.
                    TIR::CallExternFunction(_, _, p, _, _, false) => {
                        if p.iter().any(|arg| {
                            if protected_ids.contains(&arg.val) {
                                return true;
                            }
                            let mut visited = HashSet::new();
                            self.value_may_be_allocation_via_phi(func, arg.val, alloc, &mut visited)
                        }) {
                            return EscapeType::EscapesProgram;
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
            let callers: Vec<(Function, Block, SSAValue)> = self
                .builder
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
                            if let TIR::CallExternFunction(ret_id, name, _, _, ret_type, _) = i {
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
                .collect();

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
                    aliases: BTreeSet::new(), //temp
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
    ) -> Option<&TIR> {
        self.builder
            .funcs
            .iter()
            .find(|f| *f.name == function_name)
            .and_then(|f| f.body.iter().find(|b| b.id == block_id))
            .and_then(|b| b.ins.iter().find(|ins| ins.get_id() == value_id))
    }
    /// crude doubleplusungood function that tests if a given function name matches 3 known to return arrays
    fn is_array_allocation_call_name(&self, name: &str) -> bool {
        return name == "toy_malloc_arr" || name == "std::sys::argv" || name == "toy_sys_get_argv";
    }
    /// tries to determine if a function returns an array by name, this is bad however and should in future just ask the TIRBuilder
    fn function_returns_array_allocation(&self, function_name: &str) -> bool {
        let Some(func) = self.builder.funcs.iter().find(|f| *f.name == function_name) else {
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
                if self.is_array_allocation_call_name(f_box) {
                    return "toy_free_arr".to_string();
                }
                return "toy_free".to_string();
            }
            // For local function calls that return heap-allocated values (strings),
            TIR::CallLocalFunction(_, callee_name, _, _, _) => {
                if self.function_returns_array_allocation(callee_name) {
                    return "toy_free_arr".to_string();
                }
                return "toy_free".to_string();
            }
            _ => unreachable!(),
        };
    }
    /// finds all aliases and encapsulators for an allocation, including transitive alias<->encapsulator closure
    fn find_aliases_and_encapsulators(&self, alloc: &mut HeapAllocation) {
        let summary_by_func: HashMap<String, Vec<usize>> = self
            .cfg_functions
            .iter()
            .map(|cfg_f| {
                (
                    (*cfg_f.func.name).clone(),
                    cfg_f.returns_alias_of_parameter.clone(),
                )
            })
            .collect();

        let mut value_to_block: HashMap<(String, ValueId), BlockId> = HashMap::new();
        for f in &self.builder.funcs {
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

        loop {
            let mut changed = false;

            let mut new_aliases = alias_values.clone();
            for f in &self.builder.funcs {
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
                            TIR::CallLocalFunction(out_id, callee_name, params, _, _)
                            | TIR::CallExternFunction(out_id, callee_name, params, _, _, _) => {
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
            alias_values = new_aliases;

            let mut new_encapsulators = encapsulator_values.clone();
            for f in &self.builder.funcs {
                let function_name = (*f.name).clone();
                for block in &f.body {
                    for ins in &block.ins {
                        match ins {
                            TIR::CreateStructLiteral(out_id, _, params) => {
                                if params.iter().any(|param| {
                                    alias_values.contains(&(function_name.clone(), param.val))
                                }) {
                                    if new_encapsulators
                                        .insert((function_name.clone(), *out_id))
                                    {
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
                            TIR::CallExternFunction(_, callee_name, params, _, _, _)
                                if callee_name.as_ref() == "toy_write_to_arr" =>
                            {
                                if params.len() >= 2
                                    && alias_values
                                        .contains(&(function_name.clone(), params[1].val))
                                {
                                    if new_encapsulators
                                        .insert((function_name.clone(), params[0].val))
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
            for f in &self.builder.funcs {
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

            encapsulator_values = enc_alias_closure;

            if !changed {
                break;
            }
        }

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
                alloc.aliases
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
                alloc.encapsulators
                    .insert((function_name.clone(), block_id, *value_id));
            }
        }
    }
    /// runs the full pipeline to mark (or intentionally leak) a given allocation
    fn process_allocation(
        &mut self,
        alloc: &mut HeapAllocation,
        insertion_points: &mut Vec<(String, BlockId, ValueId, SSAValue, String)>,
    ) {
        let func = self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == *alloc.function)
            .unwrap();
        self.find_aliases_and_encapsulators(alloc);
        let is_param = func.params.iter().any(|p| p.val == alloc.alloc_ins.val);
        if is_param {
            return;
        }
        let cfg_func = self
            .cfg_functions
            .iter()
            .find(|f| f.func.name == func.name)
            .unwrap();
        if self.allocation_escapes(&alloc) == EscapeType::EscapesProgram {
            //at this pont let it leak, it it escapes the program
            return;
        } else if self.allocation_escapes(&alloc) == EscapeType::DoesNotEscape {
            //if in this branch, the allocation dies in ths function
            self.process_non_escaping_allocation(cfg_func, func, &alloc, insertion_points);
        } else {
            let (owning_func_name, owning_val) = self.find_owning_function(&alloc);

            let owning_func = self
                .builder
                .funcs
                .iter()
                .find(|f| *f.name == owning_func_name)
                .unwrap();

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
                owning_func,
                &owned_alloc,
                insertion_points,
            );
        }
    }
    /// Runs CTLA Analysis on the given Builder, returns a vec of functions containing the processed code, or an error.
    pub fn analyze(&mut self, builder: TirBuilder) -> Result<Vec<Function>, ToyError> {
        self.builder = builder;
        self.cfg_functions.clear();
        for f in &mut self.builder.funcs {
            let mut cfg_f = CFGFunction::new(f.to_owned());
            cfg_f.calc_cfg();
            self.cfg_functions.push(cfg_f);
        }
        self.populate_return_alias_parameter_summaries();
        let mut unique_allocations = self.builder.detect_unique_heap_allocations();
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
                .splice_free_before(name, bid, vid, val, free_name);
        }
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;
