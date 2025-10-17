use super::{Compiler, Scope};
use crate::debug;
use crate::parser::ast::Ast;
use crate::token::TypeTok;
use cranelift::prelude::*;
use cranelift_module::Module;
use std::cell::RefCell;
use std::rc::Rc;

impl Compiler {
    pub fn compile_var_reassign<M: Module>(
        &mut self,
        var_res: &Ast,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) {
        if var_res.node_type() != "VarReassign" {
            panic!("[ERROR] Expecting VarReassign, got {}", var_res);
        }
        let var_name: String;
        let new_val: Ast;
        match var_res {
            Ast::VarReassign(name, new_val_b) => {
                var_name = *name.clone();
                new_val = *new_val_b.clone()
            }
            _ => panic!("[ERROR] Expecting VarReassign, got {}", var_res),
        }
        let (var, _old_type) = scope.as_ref().borrow().get(var_name);
        let (val, _) = self.compile_expr(&new_val, _module, builder, scope);
        builder.def_var(var, val);
    }

    pub fn compile_var_dec<M: Module>(
        &mut self,
        var_dec: &Ast,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) {
        if var_dec.node_type() != "VarDec" {
            panic!(
                "[ERROR] Expected variable declarations, got {}, of type {}",
                var_dec,
                var_dec.node_type()
            );
        }
        let name: String;
        let val: Ast;
        let t_o: &TypeTok;
        match var_dec {
            Ast::VarDec(n, t, v) => {
                name = *n.clone();
                val = *v.clone();
                t_o = t;
            }
            _ => {
                panic!("[ERROR] Expected variable declarations, got {}", var_dec);
            }
        }
        let (val, _) = self.compile_expr(&val, _module, builder, scope);
        let var = Variable::new(self.var_count);
        builder.declare_var(var, types::I64);
        builder.def_var(var, val);
        self.var_count += 1;
        debug!(targets: ["compiler_verbose"], val);
        debug!(targets: ["compiler_verbose"], var);
        scope.borrow_mut().set(name, var, t_o.clone());
    }
}
