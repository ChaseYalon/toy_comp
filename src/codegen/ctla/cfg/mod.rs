use crate::codegen::tir::ir::{BlockId, Function, TIR};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EscapeType {
    EscapesProgram,
    EscapesModule,
    EscapesFunction,
    DoesNotEscape,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
///will ALWAYS share a block id with its internal block
pub struct CFGBlock {
    pub block: BlockId,
    ///id of all the blocks that could cause the block to run
    pub possible_input_blocks: Vec<BlockId>,

    ///id of all the possible blocks it could output to
    pub possible_output_blocks: Vec<BlockId>,
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
#[derive(Serialize, Deserialize, Clone)]
pub struct CFGFunction {
    pub func: Function,
    /// parameter indexes whose value may be returned (possibly via phi chains)
    pub returns_alias_of_parameter: Vec<usize>,
    /// parameter indexes whose value may be encapsulated in a returned value
    pub parameter_encapsulates: Vec<usize>,
    /// returns the idx's of any parameters who escape the program
    pub parameter_escapes: Vec<usize>,
    ///block id -> index in funcs.block
    pub block_id_to_index: HashMap<BlockId, usize>,
    pub cfg_blocks: Vec<CFGBlock>,
    ///maps a block id to the id's of all the different blocks that could input to it
    pub block_id_to_inputs: HashMap<BlockId, Vec<BlockId>>,
    pub visited_blocks: HashSet<BlockId>,
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
            parameter_encapsulates: vec![],
            block_id_to_index: id_to_idx,
            cfg_blocks: vec![],
            block_id_to_inputs: HashMap::new(),
            visited_blocks: HashSet::new(),
            parameter_escapes: vec![],
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
    pub fn calc_cfg(&mut self) {
        //build base tree - DCE covered by LLVM, they will do it better than I can anyways

        //no inputs for  start block
        self.block_id_to_inputs.insert(self.func.body[0].id, vec![]);
        self.calc_block_cfg(self.func.body[0].id);
        for b in &mut self.cfg_blocks {
            b.possible_input_blocks = self.block_id_to_inputs.get(&b.block).unwrap().to_owned();
        }
    }
}

#[cfg(test)]
mod tests;
