use std::collections::HashSet;

use crate::{
    codegen::{
        Function, SSAValue, TIR,
        tir::ir::{HeapAllocation, TirBuilder},
    },
    errors::ToyError,
};

use super::tir::ir::BlockId;

pub struct CTLA {
    builder: TirBuilder,
    /// tracks (block_id, value_id) pairs that already have a free inserted to avoid duplicates from phi-merged allocs
    freed_values: HashSet<(BlockId, usize)>,
}
#[derive(Debug, Clone)]
enum CfgNode {
    ///conditional jump, the id is for the block in question, and the two nodes are true first, false second
    ConditionalJump(BlockId, Box<CfgNode>, Box<CfgNode>),
    ///unconditional jump, block id is the block in question, and the node is the next block
    UnconditionalJump(BlockId, Box<CfgNode>),
    ///return, id is the block in question
    Return(BlockId),
    ///loop back, id is the block in question
    LoopBack(BlockId),
}
impl CTLA {
    pub fn new() -> CTLA {
        return CTLA {
            builder: TirBuilder::new(),
            freed_values: HashSet::new(),
        };
    }
    fn build_cfg_graph(&self, func: String, idx: BlockId, visited: &mut Vec<BlockId>) -> CfgNode {
        if visited.contains(&idx) {
            return CfgNode::LoopBack(idx);
        }
        visited.push(idx);
        let block = self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == func)
            .unwrap()
            .body
            .iter()
            .find(|b| b.id == idx)
            .unwrap();

        let terminator = block
            .ins
            .iter()
            .find(|ins| {
                matches!(
                    ins,
                    TIR::Ret(_, _) | TIR::JumpCond(_, _, _, _) | TIR::JumpBlockUnCond(_, _)
                )
            })
            .unwrap(); //SAFETY: Every block must have at least one terminator

        let node = match terminator {
            TIR::JumpCond(_, _, true_b_idx, false_b_idx) => CfgNode::ConditionalJump(
                idx,
                Box::new(self.build_cfg_graph(func.clone(), true_b_idx.clone(), visited)),
                Box::new(self.build_cfg_graph(func, false_b_idx.clone(), visited)),
            ),
            TIR::JumpBlockUnCond(_, jump_to_idx) => CfgNode::UnconditionalJump(
                idx,
                Box::new(self.build_cfg_graph(func, jump_to_idx.clone(), visited)),
            ),
            TIR::Ret(_, _) => CfgNode::Return(idx),
            _ => unreachable!(), //not possible, every block must end with one of those three
        };
        visited.pop();
        return node;
    }
    fn find_cfg_node(&self, node: CfgNode, target_id: BlockId) -> Option<CfgNode> {
        match &node {
            CfgNode::ConditionalJump(id, t_node, f_node) => {
                if *id == target_id {
                    return Some(node.clone());
                }
                let t_result = self.find_cfg_node(*t_node.clone(), target_id);
                if t_result.is_some() {
                    return t_result;
                }
                let f_result = self.find_cfg_node(*f_node.clone(), target_id);
                if f_result.is_some() {
                    return f_result;
                }
                return None;
            }
            CfgNode::UnconditionalJump(id, node) => {
                if *id == target_id {
                    return Some(CfgNode::UnconditionalJump(*id, node.clone()));
                }
                return self.find_cfg_node(*node.clone(), target_id);
            }
            CfgNode::Return(id) => {
                if *id == target_id {
                    return Some(node.clone());
                }
                return None;
            }
            CfgNode::LoopBack(id) => {
                if *id == target_id {
                    return Some(node.clone());
                }
                return None;
            }
        }
    }
    /// Checks if a CFG node or any of its descendants reference the given allocation
    fn node_references_allocation(&self, node: &CfgNode, alloc: &HeapAllocation) -> bool {
        let block_id = match node {
            CfgNode::ConditionalJump(id, _, _) => *id,
            CfgNode::UnconditionalJump(id, _) => *id,
            CfgNode::Return(id) => *id,
            CfgNode::LoopBack(id) => *id,
        };

        // Check if this node's block references the allocation
        let block_has_ref = alloc
            .refs
            .iter()
            .any(|(_, ref_block, _)| *ref_block == block_id);

        if block_has_ref {
            return true;
        }

        // Recursively check children
        match node {
            CfgNode::ConditionalJump(_, true_box, false_box) => {
                self.node_references_allocation(true_box, alloc)
                    || self.node_references_allocation(false_box, alloc)
            }
            CfgNode::UnconditionalJump(_, next) => self.node_references_allocation(next, alloc),
            CfgNode::Return(_) => false,
            CfgNode::LoopBack(_) => false,
        }
    }

    fn get_insertion_index(&self, func: &Function, block_id: BlockId) -> usize {
        let block = func.body.iter().find(|b| b.id == block_id).unwrap();
        // Find the first terminator
        let idx = block.ins.iter().position(|ins| {
            matches!(
                ins,
                TIR::Ret(_, _) | TIR::JumpCond(_, _, _, _) | TIR::JumpBlockUnCond(_, _)
            )
        });
        match idx {
            Some(i) => i,
            None => block.ins.len(), // Should not happen if block is well-formed
        }
    }
    fn block_returns_allocation(
        &self,
        func: &Function,
        block_id: BlockId,
        alloc: &HeapAllocation,
    ) -> bool {
        let rep = Self::find_alloc_representative(func, alloc, block_id);
        let block = func.body.iter().find(|b| b.id == block_id).unwrap();
        // Find the return instruction
        for ins in &block.ins {
            if let TIR::Ret(_, returned_val) = ins {
                // Check if the returned value is either the raw alloc or its phi representative
                if returned_val.val == alloc.alloc_ins.val || returned_val.val == rep.val {
                    return true;
                }
            }
        }
        return false;
    }

    /// Checks if `alloc` (or one of its refs) is an incoming value of a phi in `block_id`
    /// specifically from predecessor `from_block`.
    fn alloc_enters_phi_from(
        &self,
        func: &Function,
        block_id: BlockId,
        from_block: BlockId,
        alloc: &HeapAllocation,
    ) -> bool {
        let block = func.body.iter().find(|b| b.id == block_id).unwrap();
        for ins in &block.ins {
            if let TIR::Phi(_, pred_block_ids, values) = ins {
                for (pred_block, val) in pred_block_ids.iter().zip(values.iter()) {
                    if *pred_block == from_block {
                        if val.val == alloc.alloc_ins.val
                            || alloc.refs.iter().any(|r| r.2 == val.val)
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    fn alloc_type_to_free_func(&self, alloc: &HeapAllocation) -> String {
        let func = self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == *alloc.function)
            .unwrap();
        let block = func.body.iter().find(|b| b.id == alloc.block).unwrap();
        // Find the instruction by its ID, not by using the ID as an array index
        let alloc_ins = block
            .ins
            .iter()
            .find(|ins| ins.get_id() == alloc.alloc_ins.val)
            .expect("Allocation instruction not found in block");
        match alloc_ins {
            TIR::CallExternFunction(_, f_box, _, _, _) => {
                //that argv thing is hacky but I dont know how to say under the hood it calls toy_malloc_arr
                if *f_box.to_owned() == "toy_malloc_arr".to_string()
                    || "std::sys::argv".to_string() == *f_box.to_owned()
                {
                    return "toy_free_arr".to_string();
                }
                return "toy_free".to_string();
            }
            // For local function calls that return heap-allocated values (strings),
            // infer free type from the callee's returned allocation origin
            TIR::CallLocalFunction(_, callee_name, _, _, _) => {
                let callee = self
                    .builder
                    .funcs
                    .iter()
                    .find(|f| *f.name == **callee_name)
                    .unwrap();

                for callee_alloc in &callee.heap_allocations {
                    let returns_this_alloc = callee
                        .body
                        .iter()
                        .any(|b| self.block_returns_allocation(callee, b.id, callee_alloc));

                    if !returns_this_alloc {
                        continue;
                    }

                    let alloc_block = callee
                        .body
                        .iter()
                        .find(|b| b.id == callee_alloc.block)
                        .unwrap();
                    let callee_alloc_ins = alloc_block
                        .ins
                        .iter()
                        .find(|ins| ins.get_id() == callee_alloc.alloc_ins.val)
                        .unwrap();

                    if let TIR::CallExternFunction(_, f_box, _, _, _) = callee_alloc_ins {
                        if *f_box.to_owned() == "toy_malloc_arr".to_string()
                            || "std::sys::argv".to_string() == *f_box.to_owned()
                        {
                            return "toy_free_arr".to_string();
                        }
                    }

                    return "toy_free".to_string();
                }

                return "toy_free".to_string();
            }
            _ => unreachable!(),
        };
    }
    /// When an allocation flows through a phi node, the raw alloc_ins value is only valid
    /// in the block where it was created. In downstream blocks, the phi result is the correct
    /// SSA representative. This finds the phi result in `block_id` that transitively references
    /// the allocation, or falls back to alloc_ins if no phi is found.
    fn find_alloc_representative(
        func: &Function,
        alloc: &HeapAllocation,
        block_id: BlockId,
    ) -> SSAValue {
        let block = func.body.iter().find(|b| b.id == block_id).unwrap();
        for ins in &block.ins {
            if let TIR::Phi(phi_id, _, vals) = ins {
                for v in vals {
                    if v.val == alloc.alloc_ins.val || alloc.refs.iter().any(|r| r.2 == v.val) {
                        return SSAValue {
                            val: *phi_id,
                            ty: alloc.alloc_ins.ty.clone(),
                        };
                    }
                }
            }
        }
        alloc.alloc_ins.clone()
    }

    /// Inserts a free call, but uses the phi representative value when the raw alloc value
    /// is not valid in the target block, and deduplicates when multiple branch allocs
    /// reduce to the same phi.
    fn insert_free(&mut self, func: &Function, alloc: &HeapAllocation, block_id: BlockId) {
        let rep = Self::find_alloc_representative(func, alloc, block_id);
        let key = (block_id, rep.val);
        if self.freed_values.contains(&key) {
            return;
        }
        self.freed_values.insert(key);
        let idx = self.get_insertion_index(func, block_id);
        self.builder.splice_free_before(
            *func.name.clone(),
            block_id,
            idx,
            rep,
            self.alloc_type_to_free_func(alloc),
        );
    }

    fn follow_cfg_graph(
        &mut self,
        alloc_node: CfgNode,
        func: &Function,
        alloc: HeapAllocation,
    ) -> Option<()> {
        match alloc_node {
            CfgNode::Return(return_block_id) => {
                // If this block returns the allocation, don't free it - caller takes ownership
                if self.block_returns_allocation(func, return_block_id, &alloc) {
                    return Some(());
                }
                self.insert_free(func, &alloc, return_block_id);
                return Some(());
            } //for now return is the end of a graph and allocations can not be passed up the call chain
            CfgNode::ConditionalJump(block_id, true_box, false_box) => {
                // Check if either branch references the allocation
                let true_branch_refs = self.node_references_allocation(&true_box, &alloc);
                let false_branch_refs = self.node_references_allocation(&false_box, &alloc);

                if !true_branch_refs && !false_branch_refs {
                    // Neither branch references it, free it at the end of this block
                    self.insert_free(func, &alloc, block_id);
                    return Some(());
                }

                if true_branch_refs && !false_branch_refs {
                    self.follow_cfg_graph(*true_box, func, alloc.clone());
                    // Free in false branch
                    let false_block_id = match *false_box {
                        CfgNode::ConditionalJump(id, _, _) => id,
                        CfgNode::UnconditionalJump(id, _) => id,
                        CfgNode::Return(id) => id,
                        CfgNode::LoopBack(id) => id,
                    };
                    if !self.block_returns_allocation(func, false_block_id, &alloc) {
                        self.insert_free(func, &alloc, false_block_id);
                    }
                    return Some(());
                }

                if false_branch_refs && !true_branch_refs {
                    self.follow_cfg_graph(*false_box, func, alloc.clone());
                    // Free in true branch
                    let true_block_id = match *true_box {
                        CfgNode::ConditionalJump(id, _, _) => id,
                        CfgNode::UnconditionalJump(id, _) => id,
                        CfgNode::Return(id) => id,
                        CfgNode::LoopBack(id) => id,
                    };
                    if !self.block_returns_allocation(func, true_block_id, &alloc) {
                        self.insert_free(func, &alloc, true_block_id);
                    }
                    return Some(());
                }

                // Both branches reference it - follow both
                self.follow_cfg_graph(*true_box, func, alloc.clone());
                self.follow_cfg_graph(*false_box, func, alloc);
                return Some(());
            }
            CfgNode::UnconditionalJump(block_id, next) => {
                // Check if the next node references the allocation
                let next_refs = self.node_references_allocation(&next, &alloc);

                if !next_refs {
                    // Next node doesn't reference it, free it at the end of this block
                    self.insert_free(func, &alloc, block_id);
                    return Some(());
                }

                // If the next block is a Return that references the alloc only via a phi
                // whose incoming value for THIS predecessor is a DIFFERENT alloc, then
                // the alloc is not alive beyond this block on this path — free it here.
                if let CfgNode::Return(return_block_id) = next.as_ref() {
                    if !self.alloc_enters_phi_from(func, *return_block_id, block_id, &alloc) {
                        self.insert_free(func, &alloc, block_id);
                        return Some(());
                    }
                }

                // Continue following if it's still referenced
                return self.follow_cfg_graph(*next, func, alloc);
            }
            CfgNode::LoopBack(_) => Some(()),
        }
    }
    fn process_allocation(&mut self, alloc: HeapAllocation) -> Result<(), ToyError> {
        let func_name = *alloc.function.clone();
        let func = self
            .builder
            .funcs
            .clone() //SAFETY: Just extracting the name, not editing the code of the clone
            .into_iter()
            .find(|f| *f.name == func_name)
            .unwrap(); //SAFETY: Will always be in the array
        let first_block_id = func.body.first().map(|b| b.id).unwrap_or(0);
        let mut visited = Vec::new();
        let cfg_graph = self.build_cfg_graph(func_name, first_block_id, &mut visited);
        let alloc_block_id = alloc.block;
        let alloc_node = self.find_cfg_node(cfg_graph, alloc_block_id).unwrap(); //SAFETY: Should always be found
        self.follow_cfg_graph(alloc_node, &func, alloc);
        return Ok(());
    }
    pub fn analyze(&mut self, builder: TirBuilder) -> Result<Vec<Function>, ToyError> {
        self.builder = builder;
        let allocations = self.builder.detect_heap_allocations();
        for allocation in allocations {
            self.process_allocation(allocation)?;
        }
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;
