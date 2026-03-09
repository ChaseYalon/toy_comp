#![allow(unused)]
use crate::codegen::tir::ir::{BlockId, Function, SSAValue, TirBuilder, ValueId};
use crate::driver::Driver;
use crate::errors::ToyErrorType;
use crate::lexer::Lexer;
use crate::parser::ast::InfixOp;
use crate::parser::boxer::Boxer;
use crate::parser::toy_box::TBox;
use crate::token::TypeTok;
use crate::{
    codegen::tir::ir::{TIR, TirType},
    errors::ToyError,
    parser::ast::Ast,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fs;
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
    main_func_name: String,
    loop_stack: Vec<LoopContext>,
}

#[derive(Debug, Clone)]
struct LoopContext {
    continue_target: BlockId,
    break_target: BlockId,
    tracked_vars: Vec<String>,
    backedges: Vec<(BlockId, BTreeMap<String, SSAValue>)>,
}

impl AstToIrConverter {
    fn has_loop_control_for_current_loop(node: &Ast) -> bool {
        match node {
            Ast::Break(_) | Ast::Continue(_) => true,
            Ast::IfStmt(_, body, alt, _) => {
                body.iter().any(Self::has_loop_control_for_current_loop)
                    || alt
                        .as_ref()
                        .map(|a| a.iter().any(Self::has_loop_control_for_current_loop))
                        .unwrap_or(false)
            }
            Ast::WhileStmt(_, _, _) => false,
            Ast::FuncDec(_, _, _, _, _) => false,
            _ => false,
        }
    }

    fn snapshot_loop_vars(
        &self,
        scope: &Rc<RefCell<Scope>>,
        tracked_vars: &[String],
    ) -> Result<BTreeMap<String, SSAValue>, ToyError> {
        let mut snapshot = BTreeMap::new();
        for var_name in tracked_vars {
            let val = scope.as_ref().borrow().get_var(var_name)?;
            snapshot.insert(var_name.clone(), val);
        }
        Ok(snapshot)
    }

    pub fn new() -> AstToIrConverter {
        return AstToIrConverter {
            builder: TirBuilder::new(),
            global_scope: Rc::new(RefCell::new(Scope {
                parent: None,
                vars: BTreeMap::new(),
            })),
            last_val: None,
            interfaces: HashMap::new(),
            main_func_name: "user_main".to_string(),
            loop_stack: vec![],
        };
    }
    fn get_expr_type(&self, node: &Ast, scope: &Rc<RefCell<Scope>>) -> Result<TypeTok, ToyError> {
        match node {
            Ast::IntLit(_, _) => Ok(TypeTok::Int),
            Ast::BoolLit(_, _) => Ok(TypeTok::Bool),
            Ast::StringLit(_, _) => Ok(TypeTok::Str),
            Ast::FloatLit(_, _) => Ok(TypeTok::Float),
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
                _ => {
                    if let Some((_, type_tok, _, _)) =
                        self.builder.extern_funcs.get(&*n.to_string())
                    {
                        Ok(type_tok.clone())
                    } else if let Some(f) =
                        self.builder.funcs.iter().find(|f| *f.name == *n.clone())
                    {
                        match f.ret_type {
                            TirType::I64 => Ok(TypeTok::Int),
                            TirType::F64 => Ok(TypeTok::Float),
                            TirType::I1 => Ok(TypeTok::Bool),
                            TirType::Void => Ok(TypeTok::Void),
                            TirType::Ptr => Ok(TypeTok::Str),
                            _ => Ok(TypeTok::Int),
                        }
                    } else {
                        Ok(TypeTok::Int)
                    }
                }
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
            Ast::Not(_, _) => Ok(TypeTok::Bool),
            Ast::MemberAccess(target, field_name, span) => {
                let target_ty = self.get_expr_type(target, scope)?;
                match target_ty {
                    TypeTok::Struct(fields) => {
                        if let Some(field_ty) = fields.get(field_name) {
                            Ok(*field_ty.clone())
                        } else {
                            Err(ToyError::new(ToyErrorType::KeyNotOnStruct, span.clone()))
                        }
                    }
                    _ => Err(ToyError::new(ToyErrorType::VariableNotAStruct, node.span())),
                }
            }

            _ => Err(ToyError::new(ToyErrorType::TypeIdNotAssigned, node.span())),
        }
    }

    fn compile_expr(
        &mut self,
        node: Ast,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<SSAValue, ToyError> {
        let res = match node {
            Ast::IntLit(v, _) => self.builder.iconst(v, TypeTok::Int),
            Ast::BoolLit(b, _) => self.builder.iconst(if b { 1 } else { 0 }, TypeTok::Bool),
            Ast::FloatLit(f, _) => self.builder.fconst(f.into()),
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
                    InfixOp::GreaterThan,
                    InfixOp::GreaterThanEqt,
                    InfixOp::LessThanEqt,
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
                    //at this point assume it is a string expression
                    if op == InfixOp::Equals {
                        return self.builder.call_extern(
                            "toy_strequal".to_string(),
                            vec![left, right],
                            true,
                        );
                    }
                    if op == InfixOp::Plus {
                        return self.builder.call_extern(
                            "toy_concat".to_string(),
                            vec![left, right],
                            true,
                        );
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

                let is_user_defined = self
                    .builder
                    .extern_funcs
                    .get(name)
                    .map(|(_, _, _, is_builtin_extern)| !*is_builtin_extern)
                    .unwrap_or(false);

                let mut final_params = Vec::new();
                if !is_user_defined && vec!["toy_print", "toy_println"].contains(&name) {
                    if p.len() != 1 {
                        return unreachable!();
                    }
                    let ty = self.get_expr_type(&p[0], scope)?;
                    let mut is_handled = false;

                    if let TypeTok::Struct(fields) | TypeTok::StructArr(fields, _) = &ty {
                        let (fields_ref, is_arr, dimension) = match &ty {
                            TypeTok::Struct(fields) => (fields, false, 0),
                            TypeTok::StructArr(fields, d) => (fields, true, *d),
                            _ => unreachable!(),
                        };

                        let struct_ty = TypeTok::Struct(fields_ref.clone());
                        let target_tir = self.builder.type_tok_to_tir_type(struct_ty);
                        let mut struct_name = None;

                        for (name, (_, tir)) in &self.interfaces {
                            if *tir == target_tir {
                                if fields_ref.len() == self.interfaces[name].0.len()
                                    && fields_ref
                                        .keys()
                                        .all(|k| self.interfaces[name].0.contains_key(k))
                                {
                                    struct_name = Some(name.clone());
                                    break;
                                }
                            }
                        }

                        if let Some(s_name) = struct_name {
                            let method_base_name = format!("{}::to_str", s_name);
                            let mangled = Driver::mangle_name(None, &method_base_name, &[]);

                            if self.builder.funcs.iter().any(|f| *f.name == mangled)
                                || self.builder.extern_funcs.contains_key(&mangled)
                            {
                                if !is_arr {
                                    final_params.push(
                                        self.builder.call(mangled, vec![ssa_params[0].clone()])?,
                                    );
                                } else {
                                    let s = format!("String[]([]{})", dimension);
                                    let str_val = self.builder.global_string(s)?;
                                    final_params.push(str_val);
                                }
                                self.builder.inject_type_param(
                                    &TypeTok::Str,
                                    true,
                                    false,
                                    &mut final_params,
                                )?;
                                is_handled = true;
                            }
                        }
                    }

                    if !is_handled {
                        final_params.push(ssa_params[0].clone());
                        match &ty {
                            TypeTok::Int
                            | TypeTok::Bool
                            | TypeTok::Float
                            | TypeTok::Str
                            | TypeTok::IntArr(_)
                            | TypeTok::BoolArr(_)
                            | TypeTok::FloatArr(_)
                            | TypeTok::StrArr(_) => {
                                self.builder.inject_type_param(
                                    &ty,
                                    true,
                                    false,
                                    &mut final_params,
                                )?;
                            }
                            TypeTok::StructArr(_, d) => {
                                self.builder.inject_type_param(
                                    &TypeTok::StrArr(*d),
                                    true,
                                    false,
                                    &mut final_params,
                                )?;
                            }
                            _ => {
                                self.builder.inject_type_param(
                                    &TypeTok::Int,
                                    true,
                                    false,
                                    &mut final_params,
                                )?;
                            }
                        }
                    }
                } else if !is_user_defined
                    && vec![
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
                        .inject_type_param(&ty, false, false, &mut final_params)?;
                } else {
                    final_params = ssa_params;
                }

                self.builder.call(name.to_string(), final_params)
            }
            Ast::StringLit(s, _) => {
                let st = *s;
                self.builder.global_string(st)
            }
            Ast::ArrLit(ref ty, ref vals, _) => {
                let mut ssa_vals: Vec<SSAValue> = Vec::new();
                for val in vals.clone() {
                    let compiled_val = self.compile_expr(val, scope)?;
                    ssa_vals.push(compiled_val);
                }
                let len = self.compile_expr(Ast::IntLit(vals.len() as i64, node.span()), scope)?;
                let degree = match ty {
                    TypeTok::IntArr(d) => d,
                    TypeTok::BoolArr(d) => d,
                    TypeTok::StrArr(d) => d,
                    TypeTok::FloatArr(d) => d,
                    TypeTok::AnyArr(d) => d,
                    TypeTok::StructArr(_, d) => d,
                    _ => panic!("Type {:?} does not have a degree", ty),
                };
                let mut params = vec![len];
                self.builder
                    .inject_type_param(ty, false, true, &mut params)?;
                params.push(self.builder.iconst(*degree as i64, TypeTok::Int)?);
                let arr = self.builder.call("toy_malloc_arr".to_string(), params)?;
                for (i, ssa_val) in ssa_vals.iter().enumerate() {
                    let idx = self.builder.iconst(i as i64, TypeTok::Int)?;
                    let x: SSAValue = ssa_val.clone();
                    let mut write_params: Vec<SSAValue> = vec![arr.clone(), x, idx];

                    self.builder
                        .inject_type_param(&ty, false, true, &mut write_params)?;
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
                let toy_struct = self.builder.create_struct_literal(val_vec, ty.clone())?;
                let struct_size = self
                    .builder
                    .iconst(compiled_map.len() as i64 * 8, TypeTok::Int)?;
                let mut heap_struct = self.builder.call_extern(
                    "toy_malloc_struct".to_string(),
                    vec![struct_size, toy_struct],
                    true,
                )?;
                heap_struct.ty = Some(ty);
                Ok(heap_struct)
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

            Ast::Not(v, _) => {
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

        let pre_if_vars: BTreeMap<String, (SSAValue, TypeTok)> =
            scope.as_ref().borrow().vars.clone();

        let compiled_cond = self.compile_expr(cond, scope)?;
        let pre_if_block = self.builder.get_curr_block_id();
        let (true_id, false_id) = self.builder.jump_cond(compiled_cond)?;

        // --- true branch ---
        self.builder.switch_block(true_id);
        let child_scope = Scope::new_child(scope);
        for (var_name, val) in &pre_if_vars {
            child_scope.as_ref().borrow_mut().set_var(
                var_name.clone(),
                val.0.clone(),
                val.1.clone(),
            );
        }
        for ast in body {
            self.compile_stmt(ast, &child_scope)?;
        }
        let true_end_block = self.builder.get_curr_block_id();
        let true_branch_vars: BTreeMap<String, (SSAValue, TypeTok)> =
            child_scope.as_ref().borrow().vars.clone();
        let true_terminated = self.builder.curr_block_has_terminator();

        if alt.is_none() {
            //if there is no else, then the false is the merge block
            if !true_terminated {
                self.builder.jump_block_un_cond(false_id);
            }
            self.builder.switch_block(false_id);

            // insert phi nodes for variables modified in the true branch
            for (var_name, pre_val) in &pre_if_vars {
                if let Some(true_val) = true_branch_vars.get(var_name) {
                    if true_val.0 != pre_val.0 && !true_terminated {
                        let phi_id = self.builder.alloc_value_id();
                        self.builder.insert_phi(
                            false_id,
                            phi_id,
                            vec![true_end_block, pre_if_block],
                            vec![true_val.0.clone(), pre_val.0.clone()],
                        )?;
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
            }
        } else {
            let merge_id = self.builder.create_block()?;
            if !true_terminated {
                self.builder.jump_block_un_cond(merge_id);
            }

            // --- false/else branch ---
            self.builder.switch_block(false_id);
            let else_child = Scope::new_child(scope);
            for (var_name, val) in &pre_if_vars {
                else_child.as_ref().borrow_mut().set_var(
                    var_name.clone(),
                    val.0.clone(),
                    val.1.clone(),
                );
            }
            for ast in alt.unwrap() {
                self.compile_stmt(ast, &else_child)?;
            }
            let false_end_block = self.builder.get_curr_block_id();
            let false_branch_vars: BTreeMap<String, (SSAValue, TypeTok)> =
                else_child.as_ref().borrow().vars.clone();
            let false_terminated = self.builder.curr_block_has_terminator();

            if !false_terminated {
                self.builder.jump_block_un_cond(merge_id);
            }
            self.builder.switch_block(merge_id);

            // insert phi nodes for variables modified in either branch
            for (var_name, pre_val) in &pre_if_vars {
                let true_val = true_branch_vars
                    .get(var_name)
                    .map(|v| &v.0)
                    .unwrap_or(&pre_val.0);
                let false_val = false_branch_vars
                    .get(var_name)
                    .map(|v| &v.0)
                    .unwrap_or(&pre_val.0);

                if *true_val != pre_val.0 || *false_val != pre_val.0 {
                    let phi_id = self.builder.alloc_value_id();
                    self.builder.insert_phi(
                        merge_id,
                        phi_id,
                        vec![true_end_block, false_end_block],
                        vec![true_val.clone(), false_val.clone()],
                    )?;
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
        let uses_loop_control = body.iter().any(Self::has_loop_control_for_current_loop);
        let header_id = self.builder.create_block()?;
        let pre_loop_block_id = self.builder.get_curr_block_id();
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
        let latch_id = if uses_loop_control {
            Some(self.builder.create_block()?)
        } else {
            None
        };
        let continue_target = latch_id.unwrap_or(header_id);
        self.loop_stack.push(LoopContext {
            continue_target,
            break_target: merge_id,
            tracked_vars: pre_loop_vars.keys().cloned().collect(),
            backedges: vec![],
        });
        let mut non_latch_backedge_block: Option<BlockId> = None;

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

        if !self.builder.curr_block_has_terminator() {
            let tracked_vars = self
                .loop_stack
                .last()
                .map(|ctx| ctx.tracked_vars.clone())
                .unwrap_or_default();
            let snapshot = self.snapshot_loop_vars(&child_scope, &tracked_vars)?;
            if let Some(loop_ctx) = self.loop_stack.last_mut() {
                loop_ctx
                    .backedges
                    .push((self.builder.get_curr_block_id(), snapshot.clone()));
            }
            if latch_id.is_none() {
                non_latch_backedge_block = Some(self.builder.get_curr_block_id());
            }
            self.builder.jump_block_un_cond(continue_target)?;
        }

        let mut latch_var_vals: BTreeMap<String, SSAValue> = BTreeMap::new();
        if let Some(latch) = latch_id {
            self.builder.switch_block(latch);

            if let Some(loop_ctx) = self.loop_stack.last() {
                for (var_name, pre_val) in &pre_loop_vars {
                    let mut backedge_blocks = Vec::new();
                    let mut backedge_vals = Vec::new();

                    for (pred_block, snapshot) in &loop_ctx.backedges {
                        let val = snapshot
                            .get(var_name)
                            .cloned()
                            .unwrap_or_else(|| pre_val.0.clone());
                        backedge_blocks.push(*pred_block);
                        backedge_vals.push(val);
                    }

                    let latch_val = if backedge_vals.is_empty() {
                        pre_val.0.clone()
                    } else if backedge_vals.len() == 1 {
                        backedge_vals[0].clone()
                    } else {
                        let phi_id = self.builder.alloc_value_id();
                        self.builder
                            .insert_phi(latch, phi_id, backedge_blocks, backedge_vals)?;
                        SSAValue {
                            val: phi_id,
                            ty: pre_val.0.ty.clone(),
                        }
                    };

                    latch_var_vals.insert(var_name.clone(), latch_val);
                }
            }

            if !self.builder.curr_block_has_terminator() {
                self.builder.jump_block_un_cond(header_id)?;
            }
        }

        for (var_name, pre_val) in &pre_loop_vars {
            let post_val = post_loop_vars
                .get(var_name)
                .cloned()
                .unwrap_or_else(|| pre_val.clone());
            if let Some(&phi_id) = phi_id_map.get(var_name) {
                let (loop_backedge, loop_backedge_val) = if let Some(latch) = latch_id {
                    (
                        latch,
                        latch_var_vals
                            .get(var_name)
                            .cloned()
                            .unwrap_or_else(|| post_val.0.clone()),
                    )
                } else {
                    (
                        non_latch_backedge_block.unwrap_or(body_id),
                        post_val.0.clone(),
                    )
                };
                self.builder.insert_phi(
                    header_id,
                    phi_id,
                    vec![pre_loop_block_id, loop_backedge],
                    vec![pre_val.0.clone(), loop_backedge_val],
                )?;
            }
        }

        self.loop_stack.pop();
        self.builder.switch_block(merge_id);

        return Ok(());
    }
    fn compile_extern_func_dec(
        &mut self,
        node: Ast,
        _scope: &Rc<RefCell<Scope>>,
    ) -> Result<(), ToyError> {
        let (name, _, ret_type) = match node {
            Ast::ExternFuncDec(n, p, r, _) => (*n, p, r),
            _ => unreachable!(),
        };
        self.builder.register_extern_func(name, ret_type, false);
        Ok(())
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
        self.builder.switch_fn(self.main_func_name.clone())?;
        return Ok(());
    }
    fn compile_stmt(&mut self, node: Ast, scope: &Rc<RefCell<Scope>>) -> Result<(), ToyError> {
        match node {
            Ast::IntLit(_, _)
            | Ast::BoolLit(_, _)
            | Ast::InfixExpr(_, _, _, _)
            | Ast::EmptyExpr(_, _)
            | Ast::FuncCall(_, _, _)
            | Ast::VarRef(_, _)
            | Ast::StringLit(_, _)
            | Ast::ArrLit(_, _, _)
            | Ast::StructLit(_, _, _)
            | Ast::Not(_, _) => {
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
                            Some(TirType::Ptr) => 0, // String element
                            Some(TirType::I1) => 1,  // Bool element
                            Some(TirType::I64) => 2, // Int element
                            Some(TirType::F64) => 3, // Float element
                            _ => 2,                  // Default to Int
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
                            lhs.span(),
                        ));
                    }
                }
            }
            Ast::IfStmt(_, _, _, _) => self.compile_if_stmt(node, scope)?,
            Ast::WhileStmt(_, _, _) => self.compile_while_stmt(node, scope)?,
            Ast::FuncDec(_, _, _, _, _) => self.compile_func_dec(node, scope)?,
            Ast::ExternFuncDec(_, _, _, _) => self.compile_extern_func_dec(node, scope)?,
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

            Ast::ImportStmt(name, _) => {
                let path = format!("{}.toy", name.replace(".", "/"));
                if let Ok(content) = fs::read_to_string(&path) {
                    let mut l = Lexer::new();
                    if let Ok(toks) = l.lex(content) {
                        let mut b = Boxer::new();
                        if let Ok(boxes) = b.box_toks(toks) {
                            let prefix = name.replace(".", "::");
                            for b in boxes {
                                match b {
                                    TBox::ExternFuncDec(name_tok, _, ret_type, _)
                                    | TBox::FuncDec(name_tok, _, ret_type, _, _, _) => {
                                        if let Some(n) = name_tok.get_var_name() {
                                            let full_name = format!("{}::{}", prefix, n);
                                            self.builder.register_extern(
                                                full_name,
                                                match ret_type {
                                                    TypeTok::AnyArr(_)
                                                    | TypeTok::IntArr(_)
                                                    | TypeTok::BoolArr(_)
                                                    | TypeTok::StrArr(_)
                                                    | TypeTok::FloatArr(_)
                                                    | TypeTok::StructArr(_, _) => true,
                                                    TypeTok::Str => true,
                                                    _ => false,
                                                },
                                                ret_type,
                                                false,
                                                false,
                                            );
                                        }
                                    }
                                    TBox::StructInterface(n, t, _) => {
                                        let mut tir_proto: Vec<TirType> = Vec::new();
                                        let mut key_to_idx: HashMap<String, usize> = HashMap::new();
                                        let mut count: usize = 0;
                                        for (key, val) in *t {
                                            let ty = self.builder.type_tok_to_tir_type(val);
                                            key_to_idx.insert(key, count);
                                            count += 1;
                                            tir_proto.push(ty);
                                        }
                                        let full_name = format!("{}::{}", prefix, *n);
                                        let tir = self
                                            .builder
                                            .create_struct_interface(full_name.clone(), tir_proto);
                                        self.interfaces.insert(full_name, (key_to_idx, tir));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            Ast::Continue(span) => {
                let continue_target = self
                    .loop_stack
                    .last()
                    .map(|ctx| ctx.continue_target)
                    .ok_or_else(|| {
                        ToyError::new(
                            ToyErrorType::InvalidLocationForContinueStatement,
                            span.clone(),
                        )
                    })?;

                let tracked_vars = self
                    .loop_stack
                    .last()
                    .map(|ctx| ctx.tracked_vars.clone())
                    .unwrap_or_default();
                let snapshot = self.snapshot_loop_vars(scope, &tracked_vars)?;
                if let Some(loop_ctx) = self.loop_stack.last_mut() {
                    loop_ctx
                        .backedges
                        .push((self.builder.get_curr_block_id(), snapshot));
                }

                self.builder.jump_block_un_cond(continue_target)?;
            }
            Ast::Break(span) => {
                let merge_id = self
                    .loop_stack
                    .last()
                    .map(|ctx| ctx.break_target)
                    .ok_or_else(|| {
                        ToyError::new(ToyErrorType::InvalidLocationForBreakStatement, span.clone())
                    })?;
                self.builder.jump_block_un_cond(merge_id)?;
            }

            _ => todo!("Chase you have not implemented {} yet", node),
        };
        return Ok(());
    }
    fn register_extern_funcs(&mut self) {
        //everything is either void, int64_t (int) or float (double/f64)
        self.builder
            .register_extern("toy_print".to_string(), false, TypeTok::Void, true, true); //builtins.c
        self.builder
            .register_extern("toy_println".to_string(), false, TypeTok::Void, true, true);
        self.builder
            .register_extern("toy_malloc".to_string(), true, TypeTok::Str, true, true);
        self.builder
            .register_extern("toy_concat".to_string(), true, TypeTok::Str, true, true);
        self.builder
            .register_extern("toy_strequal".to_string(), false, TypeTok::Int, true, true);
        self.builder
            .register_extern("toy_strlen".to_string(), false, TypeTok::Int, true, true);
        self.builder
            .register_extern("toy_type_to_str".to_string(), true, TypeTok::Str, true, true);
        self.builder
            .register_extern("toy_type_to_bool".to_string(), false, TypeTok::Int, true, true);
        self.builder
            .register_extern("toy_type_to_int".to_string(), false, TypeTok::Int, true, true);
        self.builder
            .register_extern("toy_type_to_float".to_string(), false, TypeTok::Int, true, true); //int representation of float bits, reinterpreted with union
        self.builder
            .register_extern("toy_int_to_float".to_string(), false, TypeTok::Float, true, true);
        self.builder.register_extern(
            "toy_float_bits_to_double".to_string(),
            false,
            TypeTok::Float,
            true,
            true,
        );
        self.builder.register_extern(
            "toy_double_to_float_bits".to_string(),
            false,
            TypeTok::Int,
            true,
            true,
        );
        self.builder
            .register_extern("toy_malloc_arr".to_string(), true, TypeTok::Str, true, true);
        self.builder
            .register_extern("toy_write_to_arr".to_string(), false, TypeTok::Void, true, true);
        self.builder
            .register_extern("toy_read_from_arr".to_string(), false, TypeTok::Int, true, true);
        self.builder
            .register_extern("toy_arrlen".to_string(), false, TypeTok::Int, true, true);
        self.builder
            .register_extern("toy_input".to_string(), true, TypeTok::Str, true, true);
        self.builder
            .register_extern("toy_free".to_string(), false, TypeTok::Void, false, false); //ctla/ctla.c
        self.builder
            .register_extern("toy_free_arr".to_string(), false, TypeTok::Void, false, true);
        self.builder
            .register_extern("toy_malloc_struct".to_string(), true, TypeTok::Any, true, true);
    }
    ///ast to convert, is_main_module, and module name
    pub fn convert(
        &mut self,
        ast: Vec<Ast>,
        is_main: bool,
        module_name: &str,
    ) -> Result<Vec<Function>, ToyError> {
        self.register_extern_funcs();
        if is_main {
            self.main_func_name = "user_main".to_string();
        } else {
            let safe_name = module_name
                .replace("/", "_")
                .replace(".", "_")
                .replace(":", "_");
            self.main_func_name = format!("init_{}", safe_name);
        }

        self.builder
            .new_func(Box::new(self.main_func_name.clone()), vec![], TypeTok::Int);
        let user_main_scope = Scope::new_child(&self.global_scope);
        for node in ast {
            self.compile_stmt(node, &user_main_scope)?;
        }
        //seems bad
        let to_res = self.builder.iconst(0, TypeTok::Int)?;
        self.builder.ret(to_res);
        if !is_main
            && self.builder.funcs[self.builder.curr_func.unwrap()].body[0]
                .ins
                .len()
                == 2
        {
            //remove user main if it is empty
            self.builder.funcs.remove(self.builder.curr_func.unwrap());
        }
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;
