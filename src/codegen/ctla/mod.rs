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
struct InsRef {
    //function name
    func: Box<String>,
    block: BlockId,
    ins: u64, //should make InsId
}
enum CtlaGraphNode {
    ///represents the heap allocation coming into program scope (actual allocation is handled by stdlib) under the hood
    ///Contains a struct representing (FunctionName, BlockId, InsId) the box is the next node in the graph
    ///Alloc can have ONE AND ONLY ONE child node
    ///Also stores the HeapAllocation itself that this whole graph is for
    Alloc(InsRef, HeapAllocation, Box<Option<CtlaGraphNode>>),
    ///A reference to a given allocation, extending its lifetime through the execution of that instruction
    ///Ref can have ONE AND ONLY ONE child node
    Ref(InsRef, Box<CtlaGraphNode>),
    ///Represents ownership of the variable being passed to a function, InsRef refers to the CALL instruction, the string is the function name
    ///PassToFunc can have ONE AND ONLY ONE child node, will be inside of the CALLED function while the PassToFunc node is in the CALLER function
    PassToFunc(InsRef, Box<String>, Box<Option<CtlaGraphNode>>),
    ///Branch represents a split in the CtlaGraph where there is any conditional jump node, regardless of if the heap allocation being tracked is
    ///in that branch, if it is the Ref or other type of node should be a child, if not then the node should just point to the next ref
    Branch(
        InsRef,
        Box<Option<CtlaGraphNode>>,
        Box<Option<CtlaGraphNode>>,
    ),
    ///Represents a value being returned from a function transferring ownership to the function returned To
    ///Contains the return ins, and the name of the function being RETURNED TO
    Return(InsRef, Box<Option<String>>),
}
#[derive(Debug, Clone)]
enum CfgNode {
    ///conditional jump, the id is for the block in question, and the two nodes are true first, false second
    ConditionalJump(BlockId, Box<CfgNode>, Box<CfgNode>),
    ///unconditional jump, block id is the block in question, and the node is the next block
    UnconditionalJump(BlockId, Box<CfgNode>),
    ///return, id is the block in question
    Return(BlockId),
}
struct CtlaGraph {
    root: CtlaGraphNode, //I am sure more will be needed here
}
impl CtlaGraph {
    pub fn new(ins_ref: InsRef, alloc: HeapAllocation) -> CtlaGraph {
        return CtlaGraph {
            root: CtlaGraphNode::Alloc(ins_ref, alloc, Box::new(None)),
        };
    }
}
impl InsRef {
    pub fn new(name: Box<String>, block: BlockId, ins: u64) -> InsRef {
        return InsRef {
            func: name,
            block,
            ins,
        };
    }
}
impl CTLA {
    pub fn new() -> CTLA {
        return CTLA {
            builder: TirBuilder::new(),
        };
    }
    fn build_cfg_graph(&self, func: String, idx: BlockId) -> CfgNode {
        let last_ins = self
            .builder
            .funcs
            .iter()
            .find(|f| *f.name == func)
            .unwrap()
            .body[idx]
            .ins
            .last()
            .unwrap(); //SAFETY: There will always be more than one element, also hate all these chained methods
        return match last_ins {
            TIR::JumpCond(_, _, true_b_idx, false_b_idx) => CfgNode::ConditionalJump(
                idx,
                Box::new(self.build_cfg_graph(func.clone(), true_b_idx.clone())),
                Box::new(self.build_cfg_graph(func, false_b_idx.clone())),
            ),
            TIR::JumpBlockUnCond(_, jump_to_idx) => CfgNode::UnconditionalJump(
                idx,
                Box::new(self.build_cfg_graph(func, jump_to_idx.clone())),
            ),
            TIR::Ret(_, _) => CfgNode::Return(idx),
            _ => unreachable!(), //not possible, every block must end with one of those three
        };
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
                unreachable!() //best I can tell ths is unreachable
            }
            CfgNode::UnconditionalJump(id, node) => {
                if *id == target_id {
                    return Some(*node.clone());
                }
                return self.find_cfg_node(*node.clone(), target_id);
            }
            CfgNode::Return(id) => {
                if *id == target_id {
                    return Some(node.clone());
                }
                return None;
            }
        }
    }
    fn follow_cfg_graph(&mut self, alloc_node: CfgNode, func: &Function, alloc: HeapAllocation  ) -> Option<()> {
        match alloc_node {
            CfgNode::Return(return_block_id) => {
                //NOTE: For now I am freeing at the end of the block, in the future it should be right after the ast reference
                self.builder.splice_free_before(
                    "user_main".to_string(),
                    return_block_id,
                    func.body[return_block_id].ins.len() - 1,
                    alloc.alloc_ins,
                );
                return Some(())
            } //for now return is the end of a graph and allocations can not be passed up the call chain
            CfgNode::ConditionalJump(_, true_box, false_box) => {
                let followed_true = self.follow_cfg_graph(*true_box, func, alloc.clone());
                if followed_true.is_some(){
                    return Some(())
                }
                return self.follow_cfg_graph(*false_box, func, alloc);
            }
            CfgNode::UnconditionalJump(_, next) => {
                return self.follow_cfg_graph(*next, func, alloc);
            }
        }
    }
    fn process_allocation(&mut self, alloc: HeapAllocation) -> Result<(), ToyError> {
        //only use user_main for now
        let func = self
            .builder
            .funcs
            .clone()//SAFETY: Just extracting the name, not editing the code of the clone
            .into_iter()
            .find(|f| *f.name == "user_main")
            .unwrap(); //SAFETY: Will always be in the array
        let cfg_graph = self.build_cfg_graph("user_main".to_string(), 0);
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
