use crate::{codegen::{Function, TIR, tir::ir::{BlockId, ValueId}}, errors::ToyError};
use crate::errors::ToyErrorType;

pub struct Viewer{
    funcs: Vec<Function>
}
impl Viewer{
    pub fn new() -> Viewer {
        return Viewer { 
            funcs: vec![] 
        }
    }
    pub fn set_funcs(&mut self, funcs: Vec<Function>) {
        self.funcs = funcs;
    }
    ///give a value id, will give A COPY OF the declaratory instruction and BlockId, and the index in the block
    pub fn find_ssa_value(&self, value: ValueId) -> Result<(BlockId, TIR, usize), ToyError> {
        for func in &self.funcs {
            for block in &func.body {
                for (i, instruction) in block.ins.iter().enumerate() {
                    if instruction.get_id() == value {
                        return Ok((block.id, instruction.clone(), i))
                    }
                }
            }
        }
        return Err(ToyError::new(ToyErrorType::MissingInstruction))
    }
    ///takes the idx of a function, and will return all the heap allocations in it (blockId, ins, ins_idx)
    pub fn find_heap_allocations(&self, func_idx: usize) -> Result<Vec<(BlockId, TIR, usize)>, ToyError> {
        let mut allocations: Vec<(BlockId, TIR, usize)> = Vec::new();
        for block in &self.funcs[func_idx].body {
            for (i, ins) in block.ins.iter().enumerate() {
                match ins {
                    //right now, only external functions can be heap allocators, ie user cannot define function that returns str, arr, struct (maybe)
                    &TIR::CallExternFunction(_, _, _, true, _) => allocations.push((block.id, ins.clone(), i)),
                    _ => {} //THIS WILL CAUSE BUGS!!!!!!!!!!!!!!
                };
            }
        }
        return Ok(allocations)
    }
    //should it return a value
    ///finds all references to a given ssa value, returns the block and value of the reference
    pub fn find_ref(&self, value: ValueId) -> Vec<(BlockId, ValueId)>{
        let mut refs: Vec<(BlockId, ValueId)> = Vec::new();
        for func in &self.funcs {
            for block in &func.body {
                for ins in &block.ins {
                    match ins {
                        TIR::BoolInfix(id, l, r, _)
                        |TIR::NumericInfix(id, l, r, _) => {
                            if l.val == value {refs.push((id.clone(), block.id))}
                            if r.val == value {refs.push((id.clone(), block.id))}
                        },
                        TIR::Ret(id, val) => if val.val == value {refs.push((id.clone(), block.id))},
                        TIR::CallLocalFunction(id, _, params, _)
                        |TIR::CallExternFunction(id, _, params, _, _) => {
                            for p in params {
                                if p.val == value {
                                    refs.push((id.clone(), block.id));
                                }
                            }
                        },
                        TIR::CreateStructLiteral(id, _, vals) => {
                            for v in vals {
                                if v.val == value {
                                    refs.push((id.clone(), block.id))
                                }
                            }
                        },
                        TIR::ReadStructLiteral(id, s_ref, _) => {
                            if s_ref.val == value {
                                refs.push((id.clone(), block.id))
                            }
                        },
                        TIR::WriteStructLiteral(id, s, _, n) => {
                            if s.val == value {
                                refs.push((id.clone(), block.id))
                            }
                            if n.val == value {
                                refs.push((id.clone(), block.id))
                            }
                        },
                        TIR::Phi(id, _, vs) => {
                            for v in vs {
                                if v.val == value {
                                    refs.push((id.clone(), block.id))
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        return refs;
    }
    pub fn funcs(&self) -> Vec<Function> {
        return self.funcs.clone();
    }
}