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
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
pub mod ir;
#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    parent: Option<Rc<RefCell<Scope>>>,
    vars: BTreeMap<String, (SSAValue, TypeTok)>,
}
impl Scope {
    pub fn new_child(parent: &Rc<RefCell<Scope>>) -> Rc<RefCell<Scope>> {
        return Rc::new(RefCell::new(Scope {
            parent: Some(parent.clone()),
            vars: BTreeMap::new(),
        }));
    }
    pub fn get_var(&self, name: &str) -> Result<SSAValue, ToyError> {
        if self.vars.contains_key(name) {
            return Ok(self.vars.get(name).unwrap().0.clone());
        }
        if self.parent.is_some() {
            return self.parent.as_ref().unwrap().borrow().get_var(name);
        }
        return unreachable!();
    }
    pub fn get_var_type(&self, name: &str) -> Result<TypeTok, ToyError> {
        if self.vars.contains_key(name) {
            return Ok(self.vars.get(name).unwrap().1.clone());
        }
        if self.parent.is_some() {
            return self.parent.as_ref().unwrap().borrow().get_var_type(name);
        }
        return unreachable!();
    }
    pub fn set_var(&mut self, name: String, val: SSAValue, ty: TypeTok) {
        self.vars.insert(name, (val, ty));
    }
}
pub struct AstToIrConverter {
    pub builder: TirBuilder,
    global_scope: Rc<RefCell<Scope>>,
    last_val: Option<i64>,
    ///struct name -> ((Field Name -> idx), TirType::Struct)
    interfaces: HashMap<String, (HashMap<String, usize>, TirType)>,
}

impl AstToIrConverter {
    pub fn new() -> AstToIrConverter {
        return AstToIrConverter {
            builder: TirBuilder::new(),
            global_scope: Rc::new(RefCell::new(Scope {
                parent: None,
                vars: BTreeMap::new(),
            })),
            last_val: None,
            interfaces: HashMap::new(),
        };
    }
    fn get_expr_type(&self, node: &Ast, scope: &Rc<RefCell<Scope>>) -> Result<TypeTok, ToyError> {
        match node {
            Ast::IntLit(_) => Ok(TypeTok::Int),
            Ast::BoolLit(_) => Ok(TypeTok::Bool),
            Ast::StringLit(_, _) => Ok(TypeTok::Str),
            Ast::FloatLit(_) => Ok(TypeTok::Float),
            Ast::VarRef(n, _) => scope.as_ref().borrow().get_var_type(n),
            Ast::InfixExpr(l, r, op, _) => match op {
                InfixOp::Equals
                | InfixOp::NotEquals
                | InfixOp::LessThan
                | InfixOp::GreaterThan
                | InfixOp::LessThanEqt
                | InfixOp::GreaterThanEqt
                | InfixOp::And
                | InfixOp::Or => Ok(TypeTok::Bool),
                _ => self.get_expr_type(l, scope),
            },
            Ast::EmptyExpr(e, _) => self.get_expr_type(e, scope),
            Ast::FuncCall(n, _, _) => match n.as_str() {
                "print" | "println" | "toy_write_to_arr" => Ok(TypeTok::Void),
                "len" | "toy_strlen" | "toy_arrlen" | "toy_type_to_int" | "toy_type_to_bool"
                | "toy_type_to_float" | "toy_malloc_arr" | "toy_input" => Ok(TypeTok::Int),
                "str" | "toy_type_to_str" => Ok(TypeTok::Str),
                "int" => Ok(TypeTok::Int),
                "float" => Ok(TypeTok::Float),
                "bool" => Ok(TypeTok::Bool),
                _ => Ok(TypeTok::Int),
            },
            Ast::ArrLit(ty, _, _) => Ok(ty.clone()),
            Ast::IndexAccess(target, _, _) => {
                let target_ty = self.get_expr_type(target, scope)?;
                match target_ty {
                    TypeTok::IntArr(d) => {
                        if d == 1 {
                            Ok(TypeTok::Int)
                        } else {
                            Ok(TypeTok::IntArr(d - 1))
                        }
                    }
                    TypeTok::BoolArr(d) => {
                        if d == 1 {
                            Ok(TypeTok::Bool)
                        } else {
                            Ok(TypeTok::BoolArr(d - 1))
                        }
                    }
                    TypeTok::FloatArr(d) => {
                        if d == 1 {
                            Ok(TypeTok::Float)
                        } else {
                            Ok(TypeTok::FloatArr(d - 1))
                        }
                    }
                    TypeTok::StrArr(d) => {
                        if d == 1 {
                            Ok(TypeTok::Str)
                        } else {
                            Ok(TypeTok::StrArr(d - 1))
                        }
                    }
                    TypeTok::StructArr(fields, d) => {
                        if d == 1 {
                            Ok(TypeTok::Struct(fields))
                        } else {
                            Ok(TypeTok::StructArr(fields, d - 1))
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Ast::StructLit(_, _, _) => Ok(TypeTok::Int),
            Ast::Not(_) => Ok(TypeTok::Bool),
            Ast::MemberAccess(target, field_name, _) => {
                let target_ty = self.get_expr_type(target, scope)?;
                match target_ty {
                    TypeTok::Struct(fields) => {
                        if let Some(field_ty) = fields.get(field_name) {
                            Ok(*field_ty.clone())
                        } else {
                            Err(ToyError::new(
                                ToyErrorType::KeyNotOnStruct,
                                Some(field_name.clone()),
                            ))
                        }
                    }
                    _ => Err(ToyError::new(
                        ToyErrorType::VariableNotAStruct,
                        Some(format!("{:?}", target)),
                    )),
                }
            }

            _ => Err(ToyError::new(
                ToyErrorType::TypeIdNotAssigned,
                Some(format!("{}", node)),
            )),
        }
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
            Ast::InfixExpr(left_i, right_i, op, _) => {
                let mut left = self.compile_expr(*left_i, scope)?;
                let mut right = self.compile_expr(*right_i, scope)?;

                //implement type promotion
                if left.ty == Some(TirType::F64) && right.ty == Some(TirType::I64) {
                    right = self.builder.i_to_f(right)?;
                } else if left.ty == Some(TirType::I64) && right.ty == Some(TirType::F64) {
                    left = self.builder.i_to_f(left)?;
                }
                if left.ty == right.ty && left.ty == Some(TirType::I64) {}
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
                } else if ((left.ty == Some(TirType::I64) && right.ty == Some(TirType::I64))
                    || (left.ty == Some(TirType::F64) && right.ty == Some(TirType::F64)))
                    && op == InfixOp::Equals
                {
                    self.builder.boolean_infix(left, right, op)
                } else if (left.ty == Some(TirType::I64) && right.ty == Some(TirType::I64))
                    || (left.ty == Some(TirType::F64) && right.ty == Some(TirType::F64))
                {
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
                    unreachable!()
                };
            }
            Ast::EmptyExpr(c, _) => self.compile_expr(*c, scope),
            Ast::VarRef(n, _) => scope.as_ref().borrow().get_var(&*n),
            Ast::FuncCall(n, p, _) => {
                let mut ssa_params: Vec<SSAValue> = Vec::new();
                for param in p.clone() {
                    let compiled_param = self.compile_expr(param, scope)?;
                    ssa_params.push(compiled_param);
                }
                //map builtin names
                let name: &str = match &*n.as_str() {
                    "print" => "toy_print",
                    "println" => "toy_println",
                    "len" => {
                        if p[0].node_type() == "StringLit" || p[0].node_type() == "InfixExpr" {
                            //probably not ideal
                            "toy_strlen"
                        } else {
                            "toy_arrlen"
                        }
                    }
                    "str" => "toy_type_to_str",
                    "int" => "toy_type_to_int",
                    "float" => "toy_type_to_float",
                    "bool" => "toy_type_to_bool",
                    "input" => "toy_input",
                    _ => &*n,
                };

                let mut final_params = Vec::new();
                if vec!["toy_print", "toy_println"].contains(&name) {
                    if p.len() != 1 {
                        return unreachable!();
                    }
                    final_params.push(ssa_params[0].clone());
                    let ty = self.get_expr_type(&p[0], scope)?;
                    self.builder
                        .inject_type_param(&ty, true, &mut final_params)?;
                } else if vec![
                    "toy_type_to_str",
                    "toy_type_to_int",
                    "toy_type_to_bool",
                    "toy_type_to_float",
                ]
                .contains(&name)
                {
                    if p.len() != 1 {
                        return unreachable!();
                    }
                    final_params.push(ssa_params[0].clone());
                    let ty = self.get_expr_type(&p[0], scope)?;
                    self.builder
                        .inject_type_param(&ty, false, &mut final_params)?;
                } else {
                    final_params = ssa_params;
                }

                self.builder.call(name.to_string(), final_params)
            }
            Ast::StringLit(s, _) => {
                let st = *s;
                self.builder.global_string(st)
            }
            Ast::ArrLit(ty, vals, _) => {
                let mut ssa_vals: Vec<SSAValue> = Vec::new();
                for val in vals.clone() {
                    let compiled_val = self.compile_expr(val, scope)?;
                    ssa_vals.push(compiled_val);
                }
                let len = self.compile_expr(Ast::IntLit(vals.len() as i64), scope)?;
                let mut params = vec![len];
                self.builder.inject_type_param(&ty, false, &mut params);
                let arr = self.builder.call("toy_malloc_arr".to_string(), params)?;
                for (i, ssa_val) in ssa_vals.iter().enumerate() {
                    let idx = self.builder.iconst(i as i64, TypeTok::Int)?;
                    let x: SSAValue = ssa_val.clone();
                    let mut write_params: Vec<SSAValue> = [arr.clone(), x, idx].to_vec();
                    self.builder
                        .inject_type_param(&ty, false, &mut write_params);
                    self.builder
                        .call("toy_write_to_arr".to_string(), write_params);
                }

                return Ok(arr);
            }
            Ast::IndexAccess(target, index, _) => {
                let target_ty = self.get_expr_type(&target, scope)?;
                let elem_ty = match target_ty {
                    TypeTok::IntArr(n) => {
                        if n == 1 {
                            TypeTok::Int
                        } else {
                            TypeTok::IntArr(n - 1)
                        }
                    }
                    TypeTok::BoolArr(n) => {
                        if n == 1 {
                            TypeTok::Bool
                        } else {
                            TypeTok::BoolArr(n - 1)
                        }
                    }
                    TypeTok::StrArr(n) => {
                        if n == 1 {
                            TypeTok::Str
                        } else {
                            TypeTok::StrArr(n - 1)
                        }
                    }
                    TypeTok::FloatArr(n) => {
                        if n == 1 {
                            TypeTok::Float
                        } else {
                            TypeTok::FloatArr(n - 1)
                        }
                    }
                    TypeTok::AnyArr(n) => {
                        if n == 1 {
                            TypeTok::Any
                        } else {
                            TypeTok::AnyArr(n - 1)
                        }
                    }
                    TypeTok::StructArr(kv, n) => {
                        if n == 1 {
                            TypeTok::Struct(kv)
                        } else {
                            TypeTok::StructArr(kv, n - 1)
                        }
                    }
                    _ => unreachable!(),
                };

                let target_val = self.compile_expr(*target, scope)?;
                let idx_val = self.compile_expr(*index, scope)?;
                let read_params = vec![target_val, idx_val];
                let mut res = self
                    .builder
                    .call("toy_read_from_arr".to_string(), read_params)?;

                res.ty = Some(self.builder.type_tok_to_tir_type(elem_ty));
                Ok(res)
            }
            Ast::StructLit(interface_name, kv, _) => {
                let mut compiled_map: BTreeMap<String, SSAValue> = BTreeMap::new();
                for (key, (val, _)) in *kv {
                    compiled_map.insert(key, self.compile_expr(val, scope)?);
                }
                let mut val_vec: Vec<SSAValue> = Vec::with_capacity(compiled_map.len());
                val_vec.resize(compiled_map.len(), SSAValue { val: 0, ty: None }); //placeholders
                let (m, ty) = self.interfaces.get(&*interface_name).unwrap().clone();
                for (key, val) in m {
                    val_vec[val] = compiled_map.get(&key).unwrap().clone();
                }
                self.builder.create_struct_literal(val_vec, ty)
            }
            Ast::MemberAccess(target, field_name, _) => {
                let target_val = self.compile_expr(*target, scope)?;
                let struct_type = target_val.ty.clone().unwrap();

                let field_types = match &struct_type {
                    TirType::StructInterface(types) => types.clone(),
                    _ => return unreachable!(),
                };

                let mut field_idx: Option<usize> = None;
                let mut field_type: Option<TirType> = None;

                for (_, (field_map, iface_type)) in &self.interfaces {
                    if let TirType::StructInterface(iface_fields) = iface_type {
                        if iface_fields == &field_types {
                            if let Some(&idx) =
                                (field_map as &HashMap<String, usize>).get(&field_name)
                            {
                                field_idx = Some(idx);
                                field_type = Some(field_types[idx].clone());
                                break;
                            }
                        }
                    }
                }
                let idx = field_idx.unwrap(); // parser validated
                let ty = field_type.unwrap(); // parser validated
                self.builder.read_struct_literal(target_val, idx as u64, ty)
            }

            Ast::Not(v) => {
                let val = self.compile_expr(*v, scope)?;
                self.builder.not(val)
            }
            _ => todo!("Chase you have not implemented {} expressions yet", node),
        }?;
        return Ok(res);
    }
    fn compile_var_dec(
        &mut self,
        name: String,
        ast_val: Ast,
        ty: TypeTok,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<SSAValue, ToyError> {
        let compiled_val = self.compile_expr(ast_val, scope)?;
        scope
            .as_ref()
            .borrow_mut()
            .set_var(name, compiled_val.clone(), ty);
        return Ok(compiled_val);
    }

    fn compile_if_stmt(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        let (cond, body, alt) = match node {
            Ast::IfStmt(c, b, a, _) => (*c, b, a),
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
            Ast::WhileStmt(c, b, _) => (*c, b),
            _ => unreachable!(),
        };

        let pre_loop_vars: BTreeMap<String, (SSAValue, TypeTok)> =
            scope.as_ref().borrow().vars.clone();
        let header_id = self.builder.create_block()?;
        self.builder.jump_block_un_cond(header_id);
        self.builder.switch_block(header_id);

        let mut phi_id_map: BTreeMap<String, ValueId> = BTreeMap::new();

        for var_name in pre_loop_vars.keys().rev() {
            let phi_id = self.builder.alloc_value_id();
            phi_id_map.insert(var_name.clone(), phi_id);
        }

        for (var_name, pre_val) in &pre_loop_vars {
            if let Some(&phi_id) = phi_id_map.get(var_name) {
                scope.as_ref().borrow_mut().set_var(
                    var_name.clone(),
                    SSAValue {
                        val: phi_id,
                        ty: pre_val.0.ty.clone(),
                    },
                    pre_val.1.clone(),
                );
            }
        }

        let compiled_cond = self.compile_expr(cond.clone(), scope)?;
        let (body_id, merge_id) = self.builder.jump_cond(compiled_cond)?;

        self.builder.switch_block(body_id);
        let child_scope = Scope::new_child(scope);

        for (var_name, val) in scope.as_ref().borrow().vars.clone() {
            child_scope
                .as_ref()
                .borrow_mut()
                .set_var(var_name, val.0, val.1);
        }

        for ast in body {
            self.compile_stmt(ast, &child_scope)?;
        }

        let post_loop_vars: BTreeMap<String, (SSAValue, TypeTok)> =
            child_scope.as_ref().borrow().vars.clone();

        self.builder.jump_block_un_cond(header_id)?;

        let mut phi_instructions: Vec<TIR> = Vec::new();

        for (var_name, pre_val) in &pre_loop_vars {
            let post_val = post_loop_vars
                .get(var_name)
                .cloned()
                .unwrap_or_else(|| pre_val.clone());
            if let Some(&phi_id) = phi_id_map.get(var_name) {
                let phi_ins = TIR::Phi(
                    phi_id,
                    vec![0, body_id],
                    vec![pre_val.0.clone(), post_val.0],
                );
                phi_instructions.push(phi_ins);
            }
        }

        for phi_ins in phi_instructions.into_iter() {
            self.builder.insert_at_block_start(header_id, phi_ins)?;
        }

        self.builder.switch_block(merge_id);

        return Ok(());
    }
    fn compile_func_dec(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        let (name, params, ret_type, body) = match node {
            Ast::FuncDec(n, p, r, b, _) => (*n, p, r, b),
            _ => unreachable!(),
        };
        let func_scope = Scope::new_child(scope);
        let mut ssa_params: Vec<SSAValue> = Vec::new();
        for p in params {
            let (name, param_type) = match p {
                Ast::FuncParam(n, t, _) => (*n, t),
                _ => unreachable!(),
            };
            let ssa_v = self.builder.generic_ssa(param_type.clone());
            func_scope
                .as_ref()
                .borrow_mut()
                .set_var(name, ssa_v.clone(), param_type);
            ssa_params.push(ssa_v);
        }
        self.builder
            .new_func(Box::new(name), ssa_params, ret_type.clone());
        for stmt in body {
            self.compile_stmt(stmt, &func_scope)?;
        }
        // Add implicit void return for void functions
        if ret_type == TypeTok::Void {
            self.builder.ret(SSAValue { val: 0, ty: None });
        }
        // Switch back to user_main after compiling the function
        self.builder.switch_fn("user_main".to_string())?;
        return Ok(());
    }
    fn compile_stmt(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        match node {
            Ast::IntLit(_)
            | Ast::BoolLit(_)
            | Ast::InfixExpr(_, _, _, _)
            | Ast::EmptyExpr(_, _)
            | Ast::FuncCall(_, _, _)
            | Ast::VarRef(_, _)
            | Ast::StringLit(_, _)
            | Ast::ArrLit(_, _, _)
            | Ast::StructLit(_, _, _)
            | Ast::Not(_) => {
                let _ = self.compile_expr(node, scope)?;
            }
            Ast::VarDec(box_name, ty, box_val, _) => {
                let _ = self.compile_var_dec(*box_name, *box_val, ty, scope)?;
            }
            Ast::Assignment(lhs, rhs, _) => {
                let val = self.compile_expr(*rhs, scope)?;
                match *lhs {
                    Ast::VarRef(name, _) => {
                        let ty = scope.as_ref().borrow().get_var_type(&name)?;
                        scope.as_ref().borrow_mut().set_var(*name, val, ty);
                    }
                    Ast::IndexAccess(target, index, _) => {
                        let arr = self.compile_expr(*target, scope)?;
                        let idx = self.compile_expr(*index, scope)?;

                        let type_val = match val.ty {
                            Some(TirType::I8PTR) => 4, // String
                            Some(TirType::I1) => 5,    // Bool
                            Some(TirType::I64) => 6,   // Int
                            Some(TirType::F64) => 7,   // Float
                            _ => 2,                    // Default to Int
                        };
                        let type_param = self.builder.iconst(type_val, TypeTok::Int)?;

                        let write_params = vec![arr, val, idx, type_param];
                        self.builder
                            .call("toy_write_to_arr".to_string(), write_params)?;
                    }
                    Ast::MemberAccess(target, field_name, _) => {
                        let struct_val = self.compile_expr(*target, scope)?;
                        let struct_type = struct_val.ty.clone().unwrap();

                        let field_types = match &struct_type {
                            TirType::StructInterface(types) => types.clone(),
                            _ => unreachable!(),
                        };

                        let mut field_idx: Option<usize> = None;
                        let mut field_type: Option<TirType> = None;

                        for (_, (field_map, iface_type)) in &self.interfaces {
                            if let TirType::StructInterface(iface_fields) = iface_type {
                                if iface_fields == &field_types {
                                    if let Some(&idx) =
                                        (field_map as &HashMap<String, usize>).get(&field_name)
                                    {
                                        field_idx = Some(idx);
                                        field_type = Some(field_types[idx].clone());
                                        break;
                                    }
                                }
                            }
                        }
                        let idx = field_idx.unwrap();
                        let ty = field_type.unwrap();
                        self.builder
                            .write_struct_literal(struct_val, idx as u64, val, ty)?;
                    }
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::MalformedVariableReassign,
                            Some("Invalid assignment target".to_string()),
                        ));
                    }
                }
            }
            Ast::IfStmt(_, _, _, _) => self.compile_if_stmt(node, scope)?,
            Ast::WhileStmt(_, _, _) => self.compile_while_stmt(node, scope)?,
            Ast::FuncDec(_, _, _, _, _) => self.compile_func_dec(node, scope)?,
            Ast::Return(v, _) => {
                let ast_val = *v;
                let compiled_val = self.compile_expr(ast_val, scope)?;
                self.builder.ret(compiled_val);
            }

            Ast::StructInterface(n, t, _) => {
                let mut tir_proto: Vec<TirType> = Vec::new();
                let mut key_to_idx: HashMap<String, usize> = HashMap::new();
                let mut count: usize = 0;
                for (key, val) in *t {
                    let ty = self.builder.type_tok_to_tir_type(val);
                    key_to_idx.insert(key, count);
                    count += 1;
                    tir_proto.push(ty);
                }
                let tir = self.builder.create_struct_interface(*n.clone(), tir_proto);
                self.interfaces.insert(*n, (key_to_idx, tir));
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
        self.builder
            .register_extern("toy_free_arr".to_string(), false, TypeTok::Void);
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
