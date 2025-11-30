#![allow(unused)]
use crate::codegen::tir::ir::{BlockId, Function, SSAValue, TirBuilder, ValueId};
use crate::errors::ToyErrorType;
use crate::parser::ast::InfixOp;
use crate::token::TypeTok;
use crate::{
    codegen::tir::ir::{TIR, TirType},
    errors::ToyError,
    parser::ast::Ast,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
mod ir;
#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    parent: Option<Rc<RefCell<Scope>>>,
    vars: HashMap<String, SSAValue>,
}
impl Scope {
    pub fn new_child(parent: &Rc<RefCell<Scope>>) -> Rc<RefCell<Scope>> {
        return Rc::new(RefCell::new(Scope {
            parent: Some(parent.clone()),
            vars: HashMap::new(),
        }));
    }
    pub fn get_var(&self, name: &str) -> Result<SSAValue, ToyError> {
        if self.vars.contains_key(name) {
            return Ok(self.vars.get(name).unwrap().clone());
        }
        if self.parent.is_some() {
            return self.parent.as_ref().unwrap().borrow().get_var(name);
        }
        return Err(ToyError::new(ToyErrorType::UndefinedVariable));
    }
    pub fn set_var(&mut self, name: String, val: SSAValue) {
        self.vars.insert(name, val);
    }
}
pub struct AstToIrConverter {
    builder: TirBuilder,
    global_scope: Rc<RefCell<Scope>>,
    last_val: Option<i64>,
}

impl AstToIrConverter {
    pub fn new() -> AstToIrConverter {
        return AstToIrConverter {
            builder: TirBuilder::new(),
            global_scope: Rc::new(RefCell::new(Scope {
                parent: None,
                vars: HashMap::new(),
            })),
            last_val: None,
        };
    }
    fn compile_expr(
        &mut self,
        node: Ast,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<SSAValue, ToyError> {
        let res = match node {
            Ast::IntLit(v) => self.builder.iconst(v, TypeTok::Int),
            Ast::BoolLit(b) => self.builder.iconst(if b { 1 } else { 0 }, TypeTok::Bool),
            Ast::FloatLit(f) => self.builder.fconst(f.into()),
            Ast::InfixExpr(left_i, right_i, op) => {
                let mut left = self.compile_expr(*left_i, scope)?;
                let mut right = self.compile_expr(*right_i, scope)?;
                
                //implement type promotion
                if left.ty == Some(TirType::F64) && right.ty == Some(TirType::I64) {
                    right = self.builder.i_to_f(right)?;
                } else if left.ty == Some(TirType::I64) && right.ty == Some(TirType::F64) {
                    left = self.builder.i_to_f(left)?;
                }
                
                return if vec![
                    InfixOp::LessThan,
                    InfixOp::LessThan,
                    InfixOp::GreaterThan,
                    InfixOp::GreaterThanEqt,
                    InfixOp::GreaterThan,
                    //can be str also InfixOp::Equals,
                    InfixOp::NotEquals, //will be str in the future
                    InfixOp::And,
                    InfixOp::Or,
                ]
                .contains(&op)
                    || ((op == InfixOp::Equals)
                        && left.ty == Some(TirType::I1)
                        && right.ty == Some(TirType::I1))
                {
                    self.builder.boolean_infix(left, right, op)
                //this will cause num and str infix ops to break but I dont give a fuck
                } else if (left.ty == Some(TirType::I64) && right.ty == Some(TirType::I64)) 
                    || (left.ty == Some(TirType::F64) && right.ty == Some(TirType::F64)) {
                    self.builder.numeric_infix(left, right, op)
                } else {
                    //at this point we assume it is a string expression
                    if op == InfixOp::Equals {
                        return self
                            .builder
                            .call_extern("toy_strequal".to_string(), vec![left, right]);
                    }
                    if op == InfixOp::Plus {
                        return self
                            .builder
                            .call_extern("toy_concat".to_string(), vec![left, right]);
                    }
                    return Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType)); //should be impossible
                };
            }
            Ast::EmptyExpr(c) => self.compile_expr(*c, scope),
            Ast::VarRef(n) => scope.as_ref().borrow().get_var(&*n),
            Ast::FuncCall(n, p) => {
                let mut ssa_params: Vec<SSAValue> = Vec::new();
                for param in p {
                    let compiled_param = self.compile_expr(param, scope)?;
                    ssa_params.push(compiled_param);
                }
                // `call` checks local functions first, then extern
                self.builder.call(*n, ssa_params)
            }
            Ast::StringLit(s) => {
                let st = *s;
                self.builder.global_string(st)
            }
            _ => todo!("Chase you have not implemented {} expressions yet", node),
        }?;
        return Ok(res);
    }
    fn compile_var_dec(
        &mut self,
        name: String,
        ast_val: Ast,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<SSAValue, ToyError> {
        let compiled_val = self.compile_expr(ast_val, scope)?;
        scope
            .as_ref()
            .borrow_mut()
            .set_var(name, compiled_val.clone());
        return Ok(compiled_val);
    }
    fn compile_var_reassign(
        &mut self,
        name: String,
        ast_val: Ast,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<SSAValue, ToyError> {
        let compiled_val = self.compile_expr(ast_val, scope)?;
        scope
            .as_ref()
            .borrow_mut()
            .set_var(name, compiled_val.clone());
        return Ok(compiled_val);
    }
    fn compile_if_stmt(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        let (cond, body, alt) = match node {
            Ast::IfStmt(c, b, a) => (*c, b, a),
            _ => unreachable!(),
        };
        let compiled_cond = self.compile_expr(cond, scope)?;
        let (true_id, false_id) = self.builder.jump_cond(compiled_cond)?;
        self.builder.switch_block(true_id);
        let child_scope = Scope::new_child(scope);
        for ast in body {
            self.compile_stmt(ast, &child_scope);
        }
        //if there is no else, then the false is the merge block
        let mut merge_id: Option<BlockId> = None;
        if alt.is_none() {
            self.builder.jump_block_un_cond(false_id);
            self.builder.switch_block(false_id);
        } else {
            merge_id = Some(self.builder.create_block()?);
            self.builder.jump_block_un_cond(merge_id.unwrap());
            self.builder.switch_block(false_id);
            let else_child = Scope::new_child(scope);
            for ast in alt.unwrap() {
                self.compile_stmt(ast, &else_child);
            }
            self.builder.jump_block_un_cond(merge_id.unwrap());
            self.builder.switch_block(merge_id.unwrap());
        }

        return Ok(());
    }

    fn compile_while_stmt(
        &mut self,
        node: Ast,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<(), ToyError> {
        let (cond, body) = match node {
            Ast::WhileStmt(c, b) => (*c, b),
            _ => unreachable!(),
        };

        let pre_loop_vars: HashMap<String, SSAValue> = scope.as_ref().borrow().vars.clone();

        let header_id = self.builder.create_block()?;
        self.builder.jump_block_un_cond(header_id);
        self.builder.switch_block(header_id);

        let mut phi_id_map: HashMap<String, ValueId> = HashMap::new();
        for var_name in pre_loop_vars.keys() {
            let phi_id = self.builder.alloc_value_id();
            phi_id_map.insert(var_name.clone(), phi_id);
        }

        for (var_name, pre_val) in &pre_loop_vars {
            if let Some(phi_id) = phi_id_map.get(var_name) {
                scope.as_ref().borrow_mut().set_var(
                    var_name.clone(),
                    SSAValue {
                        val: *phi_id,
                        ty: pre_val.ty.clone(),
                    },
                );
            }
        }

        let compiled_cond = self.compile_expr(cond.clone(), scope)?;
        let (body_id, merge_id) = self.builder.jump_cond(compiled_cond)?;

        self.builder.switch_block(body_id);
        let child_scope = Scope::new_child(scope);

        for (var_name, val) in scope.as_ref().borrow().vars.clone() {
            child_scope.as_ref().borrow_mut().set_var(var_name, val);
        }

        for ast in body {
            self.compile_stmt(ast, &child_scope)?;
        }

        let post_loop_vars: HashMap<String, SSAValue> = child_scope.as_ref().borrow().vars.clone();

        self.builder.jump_block_un_cond(header_id)?;

        let mut phi_instructions: Vec<TIR> = Vec::new();

        for (var_name, pre_val) in &pre_loop_vars {
            let post_val = post_loop_vars
                .get(var_name)
                .cloned()
                .unwrap_or_else(|| pre_val.clone());
            if let Some(&phi_id) = phi_id_map.get(var_name) {
                let phi_ins = TIR::Phi(phi_id, vec![0, body_id], vec![pre_val.clone(), post_val]);
                phi_instructions.push(phi_ins);
            }
        }

        for phi_ins in phi_instructions.into_iter().rev() {
            self.builder.insert_at_block_start(header_id, phi_ins)?;
        }

        self.builder.switch_block(merge_id);

        return Ok(());
    }
    fn compile_func_dec(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        let (name, params, ret_type, body) = match node {
            Ast::FuncDec(n, p, r, b) => (*n, p, r, b),
            _ => unreachable!(),
        };
        let func_scope = Scope::new_child(scope);
        let mut ssa_params: Vec<SSAValue> = Vec::new();
        for p in params {
            let (name, param_type) = match p {
                Ast::FuncParam(n, t) => (*n, t),
                _ => unreachable!(),
            };
            let ssa_v = self.builder.generic_ssa(param_type);
            func_scope
                .as_ref()
                .borrow_mut()
                .set_var(name, ssa_v.clone());
            ssa_params.push(ssa_v);
        }
        self.builder.new_func(Box::new(name), ssa_params, ret_type);
        for stmt in body {
            self.compile_stmt(stmt, &func_scope)?;
        }
        // Switch back to user_main after compiling the function
        self.builder.switch_fn("user_main".to_string())?;
        return Ok(());
    }
    fn compile_stmt(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        match node {
            Ast::IntLit(_)
            | Ast::BoolLit(_)
            | Ast::InfixExpr(_, _, _)
            | Ast::EmptyExpr(_)
            | Ast::FuncCall(_, _)
            | Ast::VarRef(_)
            | Ast::StringLit(_) => {
                let _ = self.compile_expr(node, scope)?;
            }
            Ast::VarDec(box_name, _, box_val) => {
                let _ = self.compile_var_dec(*box_name, *box_val, scope)?;
            }
            Ast::VarReassign(boxed_name, boxed_val) => {
                let _ = self.compile_var_reassign(*boxed_name, *boxed_val, scope)?;
            }
            Ast::IfStmt(_, _, _) => self.compile_if_stmt(node, scope)?,
            Ast::WhileStmt(_, _) => self.compile_while_stmt(node, scope)?,
            Ast::FuncDec(_, _, _, _) => self.compile_func_dec(node, scope)?,
            Ast::Return(v) => {
                let ast_val = *v;
                let compiled_val = self.compile_expr(ast_val, scope)?;
                self.builder.ret(compiled_val);
            }
            _ => todo!("Chase you have not implemented {} yet", node),
        };
        return Ok(());
    }
    fn register_extern_funcs(&mut self) {
        //everything is either void, int64_t (int) or float (double/f64)
        self.builder
            .register_extern("toy_print".to_string(), false, TypeTok::Void); //builtins.c
        self.builder
            .register_extern("toy_println".to_string(), false, TypeTok::Void);
        self.builder
            .register_extern("toy_malloc".to_string(), true, TypeTok::Int);
        self.builder
            .register_extern("toy_concat".to_string(), true, TypeTok::Int);
        self.builder
            .register_extern("toy_strequal".to_string(), false, TypeTok::Int);
        self.builder
            .register_extern("toy_strlen".to_string(), false, TypeTok::Int);
        self.builder
            .register_extern("toy_type_to_str".to_string(), true, TypeTok::Int);
        self.builder
            .register_extern("toy_type_to_bool".to_string(), false, TypeTok::Int);
        self.builder
            .register_extern("toy_type_to_int".to_string(), false, TypeTok::Int);
        self.builder
            .register_extern("toy_type_to_float".to_string(), false, TypeTok::Int); //int representation of float bits, reinterpreted with union
        self.builder
            .register_extern("toy_int_to_float".to_string(), false, TypeTok::Float);
        self.builder.register_extern(
            "toy_float_bits_to_double".to_string(),
            false,
            TypeTok::Float,
        );
        self.builder
            .register_extern("toy_double_to_float_bits".to_string(), false, TypeTok::Int);
        self.builder
            .register_extern("toy_malloc_arr".to_string(), true, TypeTok::Int);
        self.builder
            .register_extern("toy_write_to_arr".to_string(), false, TypeTok::Void);
        self.builder
            .register_extern("toy_read_from_arr".to_string(), false, TypeTok::Int);
        self.builder
            .register_extern("toy_arrlen".to_string(), false, TypeTok::Int);
        self.builder
            .register_extern("toy_input".to_string(), true, TypeTok::Int);
        self.builder
            .register_extern("toy_free".to_string(), false, TypeTok::Void); //ctla/ctla.c
    }
    pub fn convert(&mut self, ast: Vec<Ast>) -> Result<Vec<Function>, ToyError> {
        self.register_extern_funcs();

        self.builder
            .new_func(Box::new("user_main".to_string()), vec![], TypeTok::Int);
        let user_main_scope = Scope::new_child(&self.global_scope);
        for node in ast {
            self.compile_stmt(node, &user_main_scope)?;
        }
        //seems bad
        let to_res = self.builder.iconst(0, TypeTok::Int)?;
        self.builder.ret(to_res);
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;
