use crate::{
    codegen::{
        Function, TIR,
        tir::ir::{HeapAllocation, TirBuilder},
    },
    errors::ToyError,
};

use super::tir::ir::BlockId;

pub struct CTLA {
    builder: TirBuilder,
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
        };
    }
    fn build_cfg_graph(
        &self,
        func: String,
        idx: BlockId,
        visited: &mut Vec<BlockId>,
    ) -> CfgNode {
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

        // Find the first terminator instruction (Ret, JumpCond, or JumpBlockUnCond)
        // This handles cases where a return is followed by unreachable code
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
                if *f_box.to_owned() == "toy_malloc_arr".to_string() {
                    return "toy_free_arr".to_string();
                }
                return "toy_free".to_string();
            }
            _ => unreachable!(),
        };
    }
    fn follow_cfg_graph(
        &mut self,
        alloc_node: CfgNode,
        func: &Function,
        alloc: HeapAllocation,
    ) -> Option<()> {
        match alloc_node {
            CfgNode::Return(return_block_id) => {
                let idx = self.get_insertion_index(func, return_block_id);
                self.builder.splice_free_before(
                    *func.name.clone(),
                    return_block_id,
                    idx,
                    alloc.clone().alloc_ins,
                    self.alloc_type_to_free_func(&alloc),
                );
                return Some(());
            } //for now return is the end of a graph and allocations can not be passed up the call chain
            CfgNode::ConditionalJump(block_id, true_box, false_box) => {
                // Check if either branch references the allocation
                let true_branch_refs = self.node_references_allocation(&true_box, &alloc);
                let false_branch_refs = self.node_references_allocation(&false_box, &alloc);

                if !true_branch_refs && !false_branch_refs {
                    // Neither branch references it, free it at the end of this block
                    let idx = self.get_insertion_index(func, block_id);
                    self.builder.splice_free_before(
                        *func.name.clone(),
                        block_id,
                        idx,
                        alloc.clone().alloc_ins,
                        self.alloc_type_to_free_func(&alloc),
                    );
                    return Some(());
                }

                // If only one branch references it, follow only that branch
                // If both branches reference it, we need to free in both paths
                if true_branch_refs && !false_branch_refs {
                    self.follow_cfg_graph(*true_box, func, alloc.clone());
                    // Free in false branch
                    let false_block_id = match *false_box {
                        CfgNode::ConditionalJump(id, _, _) => id,
                        CfgNode::UnconditionalJump(id, _) => id,
                        CfgNode::Return(id) => id,
                        CfgNode::LoopBack(id) => id,
                    };
                    self.builder.splice_free_before(
                        *func.name.clone(),
                        false_block_id,
                        0, // Insert at start of block
                        alloc.clone().alloc_ins,
                        self.alloc_type_to_free_func(&alloc),
                    );
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
                    self.builder.splice_free_before(
                        *func.name.clone(),
                        true_block_id,
                        0, // Insert at start of block
                        alloc.clone().alloc_ins,
                        self.alloc_type_to_free_func(&alloc),
                    );
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
                    let idx = self.get_insertion_index(func, block_id);
                    self.builder.splice_free_before(
                        *func.name.clone(),
                        block_id,
                        idx,
                        alloc.clone().alloc_ins,
                        self.alloc_type_to_free_func(&alloc),
                    );
                    return Some(());
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
