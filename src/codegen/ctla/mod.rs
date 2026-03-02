use itertools::Itertools;

use crate::{
    codegen::{
        SSAValue,
        tir::ir::{Block, Function, HeapAllocation, TIR, TirBuilder, ValueId},
    },
    errors::ToyError,
};
use std::collections::{HashMap, HashSet};

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
            block_id_to_index: id_to_idx,
            cfg_blocks: vec![],
            block_id_to_inputs: HashMap::new(),
            visited_blocks: HashSet::new(),
        };
    }

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

impl CTLA {
    pub fn new() -> CTLA {
        return CTLA {
            builder: TirBuilder::new(),
            cfg_functions: vec![],
        };
    }
    ///NOTE: You CANNOT mutate ths value it is a copy not the original
    fn get_ins(&self, func: String, block: BlockId, ins_idx: usize) -> &TIR {
        return &self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == func)
            .unwrap()
            .body
            .iter()
            .find(|b| b.id == block)
            .unwrap()
            .ins[ins_idx];
    }
    fn allocation_escapes(&self, alloc: &HeapAllocation) -> EscapeType {
        let func = self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == *alloc.function)
            .unwrap();
        let is_param = func.params.iter().any(|p| p.val == alloc.alloc_ins.val);
        //params always freed by the caller
        if is_param {
            return EscapeType::EscapesFunction;
        }

        //alloc is returned
        for b in &func.body {
            if let Some(TIR::Ret(ret_id, a)) = b.ins.last() {
                if alloc
                    .refs
                    .iter()
                    .any(|(_, block_id, value_id)| *block_id == b.id && *value_id == *ret_id)
                {
                    return EscapeType::EscapesFunction;
                }
                if *a == alloc.alloc_ins {
                    return EscapeType::EscapesFunction;
                }
                if let Some(TIR::Phi(_, _, vals)) = b.ins.iter().find(|ins| ins.get_id() == a.val) {
                    if vals.iter().any(|v| *v == alloc.alloc_ins) {
                        return EscapeType::EscapesFunction;
                    }
                }
            }
        }

        for b in &func.body {
            for i in &b.ins {
                match i {
                    //only extern func calls are considered escapes because in regular func calls, everything s passed by reference.
                    TIR::CallExternFunction(_, _, p, _, _, false) => {
                        if p.contains(&alloc.alloc_ins) {
                            return EscapeType::EscapesProgram;
                        }
                    }
                    _ => continue,
                };
            }
        }

        return EscapeType::DoesNotEscape;
    }
    fn mark_if_block_or_children_must_live(
        &self,
        cfg_f: &CFGFunction,
        bid: BlockId,
        alloc: &HeapAllocation,
        keep_alive_list: &mut Vec<BlockId>,
        visited: &mut HashSet<BlockId>,
    ) {
        if visited.contains(&bid) {
            return;
        }
        visited.insert(bid);

        for (_, b, _) in &alloc.refs {
            if b == &bid {
                keep_alive_list.push(bid);
            }
        }
        let cfg_block = cfg_f.cfg_blocks.iter().find(|b| b.block == bid).unwrap();
        for child in &cfg_block.possible_output_blocks {
            self.mark_if_block_or_children_must_live(
                cfg_f,
                *child,
                alloc,
                keep_alive_list,
                visited,
            );
        }
    }

    fn block_children_reference_allocation(
        &self,
        func: &CFGFunction,
        cfg_b: &CFGBlock,
        alloc: &HeapAllocation,
        visited: &mut HashSet<BlockId>,
        is_root: bool,
    ) -> bool {
        if visited.contains(&cfg_b.block) { return false; }
        visited.insert(cfg_b.block);
        if cfg_b.possible_output_blocks.is_empty() { return false; }
        
        // only check refs in non-root blocks (successors, not the candidate block itself)
        if !is_root {
            for (_, b, _) in &alloc.refs {
                if b == &cfg_b.block { return true; }
            }
        }
        
        for possible_output_block in &cfg_b.possible_output_blocks {
            let child = func.cfg_blocks.iter()
                .find(|b| b.block == *possible_output_block).unwrap();
            if self.block_children_reference_allocation(func, child, alloc, visited, false) {
                return true;
            }
        }
        false
    }
    fn find_owning_function(&self, alloc: &HeapAllocation) -> (String, SSAValue) {
        let mut current_func = alloc.function.clone();
        let mut current_val = alloc.alloc_ins.clone();

        loop {
            // find all callers that receive a value from current_func via CallLocalFunction
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
        let Some(origin_cfg_block) = cfg_func.cfg_blocks.iter().find(|b| b.block == origin_block_id) else {
            return;
        };

        let mut visited: HashSet<BlockId> = HashSet::new();
        let has_child_refs = self.block_children_reference_allocation(
            cfg_func,
            origin_cfg_block,
            alloc,
            &mut visited,
            true,
        );

        if !has_child_refs {
            if let Some(origin_block) = func.body.iter().find(|b| b.id == origin_block_id) {
                insertion_points.push((
                    *func.name.clone(),
                    origin_block_id,
                    origin_block.ins.len() - 1,
                    alloc.alloc_ins.clone(),
                    free_func,
                ));
            }
            return;
        }

        // Conservative fallback: if value is defined in entry block, free at function return blocks.
        if origin_block_id == func.body[0].id {
            for block in &func.body {
                if matches!(block.ins.last(), Some(TIR::Ret(_, _))) {
                    insertion_points.push((
                        *func.name.clone(),
                        block.id,
                        block.ins.len() - 1,
                        alloc.alloc_ins.clone(),
                        free_func.clone(),
                    ));
                }
            }
        }
    }
    fn function_has_ssa(&self, func: &Function, value_id: ValueId) -> bool {
        if func.params.iter().any(|p| p.val == value_id) {
            return true;
        }
        func.body
            .iter()
            .any(|b| b.ins.iter().any(|ins| ins.get_id() == value_id))
    }
    fn get_alloc_ins(&self, function_name: &str, block_id: BlockId, value_id: ValueId) -> Option<&TIR> {
        self.builder
            .funcs
            .iter()
            .find(|f| *f.name == function_name)
            .and_then(|f| f.body.iter().find(|b| b.id == block_id))
            .and_then(|b| b.ins.iter().find(|ins| ins.get_id() == value_id))
    }
    fn is_array_allocation_call_name(&self, name: &str) -> bool {
        name == "toy_malloc_arr" || name == "std::sys::argv" || name == "toy_sys_get_argv"
    }
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
        false
    }
    fn alloc_type_to_free_func(&self, alloc: &HeapAllocation) -> String {
        let alloc_ins = self
            .get_alloc_ins(&alloc.function, alloc.block, alloc.alloc_ins.val);

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
            // we use toy_free since they return string pointers
            TIR::CallLocalFunction(_, callee_name, _, _, _) => {
                if self.function_returns_array_allocation(callee_name) {
                    return "toy_free_arr".to_string();
                }
                return "toy_free".to_string();
            }
            _ => unreachable!(),
        };
    }
    fn process_allocation(
        &self,
        alloc: HeapAllocation,
        insertion_points: &mut Vec<(String, BlockId, ValueId, SSAValue, String)>,
    ) {
        if *alloc.function == "std::sys::argv" {
            return;
        }
        let func = self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == *alloc.function)
            .unwrap();
        
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
    pub fn analyze(&mut self, builder: TirBuilder) -> Result<Vec<Function>, ToyError> {
        self.builder = builder;
        for f in &mut self.builder.funcs {
            let mut cfg_f = CFGFunction::new(f.to_owned());
            cfg_f.calc_cfg();
            self.cfg_functions.push(cfg_f);
        }
        let unique_allocations = self.builder.detect_unique_heap_allocations();
        let mut insertion_points: Vec<(String, BlockId, ValueId, SSAValue, String)> = vec![];
        for a in unique_allocations {
            self.process_allocation(a, &mut insertion_points);
        }
        eprintln!("[DEBUG] {:?}", insertion_points);
        // in analyze, before the splice loop
        let dedup_set: HashSet<_> = insertion_points.into_iter().collect();
        insertion_points = dedup_set.into_iter().collect();
        for (name, bid, vid, val, free_name) in insertion_points {
            self.builder
                .splice_free_before(name, bid, vid, val, free_name);
        }
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;
