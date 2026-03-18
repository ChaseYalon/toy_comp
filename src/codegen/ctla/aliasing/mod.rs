use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::codegen::ctla::cfg::CFGFunction;
use crate::codegen::tir::ir::{BlockId, Function, HeapAllocation, TIR, TirBuilder, ValueId};
pub struct AliasAndEncapsulationTracker {
    builder: Rc<RefCell<TirBuilder>>,
    aliases: HashSet<(String, ValueId)>,
    encapsulators: HashSet<(String, ValueId)>,
}
impl<'a> AliasAndEncapsulationTracker {
    pub fn new(builder: &Rc<RefCell<TirBuilder>>) -> AliasAndEncapsulationTracker {
        return AliasAndEncapsulationTracker {
            builder: Rc::clone(builder),
            aliases: HashSet::new(),
            encapsulators: HashSet::new(),
        };
    }
    pub fn has_alias(&self, f_name: String, value: ValueId)->bool{
        return self.aliases.contains(&(f_name, value));
    }
    pub fn has_encapsulator(&self, f_name: String, value: ValueId) -> bool {
        return self.encapsulators.contains(&(f_name, value));
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
        self.aliases = alias_values.clone();
        self.encapsulators = encapsulator_values.clone();
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
