use crate::{
    codegen::{Function, tir::ir::TirBuilder},
    errors::ToyError,
};

use super::{SSAValue, tir::ir::BlockId};

pub struct CTLA {
    builder: TirBuilder,
}
struct InsRef{
    //function name
    func: Box<String>,
    block: BlockId,
    ins: u64 //should make InsId
}
enum CtlaGraphNode {
    ///represents the heap allocation coming into program scope (actual allocation is handled by stdlib) under the hood
    ///Contains a struct representing (FunctionName, BlockId, InsId) the box is the next node in the graph
    ///Alloc can have ONE AND ONLY ONE child node
    Alloc(InsRef, Box<CtlaGraphNode>),
    ///A reference to a given allocation, extending its lifetime through the execution of that instruction
    ///Ref can have ONE AND ONLY ONE child node
    Ref(InsRef, Box<CtlaGraphNode>),
    ///Represents ownership of the variable being passed to a function, InsRef refers to the CALL instruction, the string is the function name
    ///PassToFunc can have ONE AND ONLY ONE child node, will be inside of the CALLED function while the PassToFunc node is in the CALLER function
    PassToFunc(InsRef, Box<String>, Box<CtlaGraphNode>),
    ///Branch represents a split in the CtlaGraph where there is any conditional jump node, regardless of if the heap allocation being tracked is
    ///in that branch, if it is the Ref or other type of node should be a child, if not then the node should just point to the next ref
    Branch(InsRef, Box<CtlaGraphNode>, Box<CtlaGraphNode>)

}
impl CTLA {
    pub fn new() -> CTLA {
        return CTLA {
            builder: TirBuilder::new(),
        };
    }
    pub fn analyze(&mut self, builder: TirBuilder) -> Result<Vec<Function>, ToyError> {
        self.builder = builder;
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;
