#![allow(unused)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::codegen::tir::ir::{Function, SSAValue, TirBuilder};
use crate::errors::ToyErrorType;
use crate::parser::ast::InfixOp;
use crate::token::TypeTok;
use crate::{
    codegen::tir::ir::{TIR, TirType},
    errors::ToyError,
    parser::ast::Ast,
};
mod ir;
#[derive(Debug, Clone, PartialEq)]
pub struct Scope{
    parent: Option<Rc<RefCell<Scope>>>,
    vars: HashMap<String, SSAValue>
}
impl Scope {
    pub fn new_child(parent: &Rc<RefCell<Scope>>) -> Rc<RefCell<Scope>>{
        return Rc::new(RefCell::new(Scope { 
            parent: Some(parent.clone()), 
            vars: HashMap::new() 
        }))
    }
    pub fn get_var(&self, name: &str) -> Result<SSAValue, ToyError> {
        if self.vars.contains_key(name) {
            return Ok(self.vars.get(name).unwrap().clone());
        }
        if self.parent.is_some() {
            return self.parent.as_ref().unwrap().borrow().get_var(name);
        }
        return Err(ToyError::new(ToyErrorType::UndefinedVariable))
    }
    pub fn set_var(&mut self, name: String, val: SSAValue) {
        self.vars.insert(name, val);
    }
}
pub struct AstToIrConverter {
    builder: TirBuilder,
    global_scope: Rc<RefCell<Scope>>
}

impl AstToIrConverter {
    pub fn new() -> AstToIrConverter {
        return AstToIrConverter {
            builder: TirBuilder::new(),
            global_scope: Rc::new(RefCell::new(Scope {
                parent: None,
                vars: HashMap::new()
            }))
        };
    }
    fn compile_expr(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<SSAValue, ToyError> {
        return match node {
            Ast::IntLit(v) => self.builder.iconst(v, TypeTok::Int),
            Ast::BoolLit(b) => self.builder.iconst(if b { 1 } else { 0 }, TypeTok::Bool),
            Ast::InfixExpr(left_i, right_i, op) => {
                let left = self.compile_expr(*left_i, scope)?;
                let right = self.compile_expr(*right_i, scope)?;
                return if vec![
                    InfixOp::LessThan,
                    InfixOp::LessThan,
                    InfixOp::GreaterThan,
                    InfixOp::GreaterThanEqt,
                    InfixOp::GreaterThan,
                    InfixOp::Equals,
                    InfixOp::NotEquals,
                    InfixOp::And,
                    InfixOp::Or,
                ]
                .contains(&op)
                {
                    self.builder.boolean_infix(left, right, op)
                } else {
                    self.builder.numeric_infix(left, right, op)
                };
            },
            Ast::VarRef(n) => {
                scope.as_ref().borrow().get_var(&*n)
            }
            _ => todo!("Chase you have not implemented {} expressions yet", node),
        };
    }
    fn compile_var_dec(&mut self, name: String, ast_val: Ast, scope: &Rc<RefCell<Scope>>) -> Result<SSAValue, ToyError> {
        let compiled_val = self.compile_expr(ast_val, scope)?;
        scope.as_ref().borrow_mut().set_var(name, compiled_val.clone());
    return Ok(compiled_val);
    }
    fn compile_var_reassign(&mut self, name: String, ast_val: Ast, scope: &Rc<RefCell<Scope>> ) -> Result<SSAValue, ToyError> {
        let compiled_val = self.compile_expr(ast_val, scope)?;
        scope.as_ref().borrow_mut().set_var(name, compiled_val.clone());
        return Ok(compiled_val);
    }
    fn compile_stmt(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        match node {
            Ast::IntLit(_) => self.compile_expr(node, scope)?,
            Ast::BoolLit(_) => self.compile_expr(node, scope)?,
            Ast::InfixExpr(_, _, _) => self.compile_expr(node, scope)?,
            Ast::VarDec(box_name, _, box_val) => self.compile_var_dec(*box_name, *box_val, scope)?,
            Ast::VarRef(_) => self.compile_expr(node, scope)?,
            Ast::VarReassign(boxed_name, boxed_val) => self.compile_var_reassign(*boxed_name, *boxed_val, scope)?,
            _ => todo!("Chase you have not implemented {} yet", node),
        };
        return Ok(())
    }
    pub fn convert(&mut self, ast: Vec<Ast>) -> Result<Vec<Function>, ToyError> {
        self.builder
            .new_func(Box::new("user_main".to_string()), vec![], TypeTok::Int);
        let user_main_scope = Scope::new_child(&self.global_scope);
        for node in ast {
            self.compile_stmt(node, &user_main_scope)?;
        }
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;
