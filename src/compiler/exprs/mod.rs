use super::Compiler;
use super::Scope;
use crate::parser::ast::Ast;
use crate::token::TypeTok;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use cranelift::prelude::*;
use cranelift_module::DataDescription;
use cranelift_module::Module;
use crate::errors::ToyError;

mod arrs;
mod infix;
mod structs;
impl Compiler {
    pub fn inject_type_param<M: Module>(
        &self,
        t: &TypeTok,
        inject_dimension: bool,
        _module: &M,
        builder: &mut FunctionBuilder<'_>,
        param_values: &mut Vec<Value>,
    )-> Result<(), ToyError> {
        let (n, degree) = match t {
            &TypeTok::Str => (0, 0),
            &TypeTok::Bool => (1, 0),
            &TypeTok::Int => (2, 0),
            &TypeTok::Float => (3, 0),
            &TypeTok::StrArr(n) => (4, n),
            &TypeTok::BoolArr(n) => (5, n),
            &TypeTok::IntArr(n) => (6, n),
            &TypeTok::FloatArr(n) => (7, n),
            _ => return Err(ToyError::TypeIdNotAssigned),
        };
        let v = builder.ins().iconst(types::I64, n);
        param_values.push(v);
        if inject_dimension {
            let d = builder.ins().iconst(types::I64, degree as i64);
            param_values.push(d);
        }
        return Ok(())
    }
    fn compile_func_call<M: Module>(
        &mut self,
        func_name: String,
        params: &Vec<Ast>,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<(Value, TypeTok), ToyError> {
        let temp = self.funcs.clone();
        let name: String;
        if func_name == "len".to_string() {
            let (_, t) = self.compile_expr(&params[0].clone(), module, builder, scope)?; //this is so wasteful
            let f_param_type = t.clone();
            if f_param_type == TypeTok::Str {
                name = "strlen".to_string();
            } else {
                name = "arrlen".to_string();
            }
        } else {
            name = func_name;
        }
        let o_func = temp.get(&name);
        if o_func.is_none() {
            return Err(ToyError::UndefinedFunction);
        }
        let mut param_values: Vec<Value> = Vec::new();
        let mut last_type: TypeTok = TypeTok::Str;
        let (ret_type, id, param_names) = o_func.unwrap();
        for (i, p) in params.iter().enumerate() {
            let mut g_param_name: String = "".to_string();
            if let Ast::FuncParam(param_name_b, _) = p {
                let param_name = *param_name_b.clone();
                g_param_name = param_name.clone();
                self.current_struct_name = Some(param_name.clone());
            }
            if p.node_type() == "StructLit" {
                let s_name = match p.clone() {
                    Ast::StructLit(n, _) => *n,
                    _ => unreachable!(),
                };
                self.current_struct_name = Some(s_name);
            }
            let (v, t) = self.compile_expr(&p.clone(), module, builder, scope)?;
            last_type = t.clone();

            if t.type_str() == "Struct" {
                let (kv, _old_var) = scope.borrow().get_unresolved_struct(param_names[i].clone());
                let interface_name = scope.borrow().find_interface_name_with_kv(&kv).unwrap();
                if let Ast::FuncParam(param_name_b, _) = p {
                    let param_name = *param_name_b.clone();
                    let new_var = Variable::new(self.var_count);
                    self.var_count += 1;
                    builder.declare_var(new_var, types::I64);
                    scope
                        .borrow_mut()
                        .set_struct(param_name, interface_name, new_var);
                    println!("Making variable type: {:?}", v.type_id());
                    builder.def_var(new_var, v);
                }
            }
            param_values.push(v);
        }

        if name == "str".to_string()
            || name == "bool".to_string()
            || name == "int".to_string()
            || name == "float".to_string()
        {
            self.inject_type_param(&last_type, false, module, builder, &mut param_values)?;
        }
        if name == "print".to_string() || name == "println".to_string() {
            self.inject_type_param(&last_type, true, module, builder, &mut param_values)?;
        }
        let func_ref = module.declare_func_in_func(id.clone(), builder.func);

        let call_inst = builder.ins().call(func_ref, &param_values.as_slice());
        let results = builder.inst_results(call_inst);
        if results.len() > 0 {
            let ret_val = results[0];

            return Ok((ret_val, ret_type.clone()));
        } else {
            //This is a dummy, should not be used
            return Ok((builder.ins().iconst(types::I64, 0), TypeTok::Void));
        }
    }
    fn compile_string_lit<M: Module>(
        &mut self,
        string_value: String,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<(Value, TypeTok), ToyError> {
        let data_id = module
            .declare_anonymous_data(false, false)?;

        //Create null terminated string in mem
        let mut data_desc = DataDescription::new();
        let mut string_bytes = string_value.as_bytes().to_vec();
        string_bytes.push(0); // null terminator
        data_desc.define(string_bytes.into_boxed_slice());
        module
            .define_data(data_id, &data_desc)?;

        // Get a global value reference to the data
        let data_gv = module.declare_data_in_func(data_id, builder.func);
        let string_ptr = builder.ins().global_value(types::I64, data_gv);

        // Call toy_malloc with the string pointer
        let malloc_func = match self.funcs.get("malloc") {
            Some(malloc_func) => malloc_func,
            None => return Err(ToyError::InternalFunctionUndefined)
        };
        let func_ref = module.declare_func_in_func(malloc_func.1, builder.func);
        let call_inst = builder.ins().call(func_ref, &[string_ptr]);
        let results = builder.inst_results(call_inst);
        let heap_ptr = results[0];

        Ok((heap_ptr, TypeTok::Str))
    }
    pub fn compile_expr<M: Module>(
        &mut self,
        expr: &Ast,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<(Value, TypeTok), ToyError> {
        let to_ret:(Value, TypeTok) = match expr {
            Ast::FuncCall(b_name, params) => {
                self.compile_func_call(*b_name.clone(), &params, _module, builder, scope)?
            }
            Ast::ArrRef(a, indices) => {
                self.compile_arr_ref(*a.clone(), indices, _module, builder, scope)?
            }

            Ast::ArrLit(t, val) => (
                self.compile_arr_lit(val, _module, builder, scope)?,
                t.clone(),
            ),
            Ast::IntLit(n) => (builder.ins().iconst(types::I64, *n), TypeTok::Int),
            Ast::FloatLit(f) => {
                let float = *f;
                (
                    builder.ins().iconst(types::I64, float.to_bits() as i64),
                    TypeTok::Float,
                )
            }
            Ast::StructLit(n, bkv) => {
                self.compile_struct_lit(*n.clone(), &*bkv, _module, builder, scope)?
            }
            Ast::StructRef(s_name, keys) => {
                self.compile_struct_ref(*s_name.clone(), keys, _module, builder, scope)?
            }
            Ast::BoolLit(b) => {
                let is_true: i64 = if *b { 1 } else { 0 };
                (builder.ins().iconst(types::I64, is_true), TypeTok::Bool)
            }
            Ast::StringLit(s) => self.compile_string_lit(*s.clone(), _module, builder)?,
            Ast::EmptyExpr(child) => self.compile_expr(child, _module, builder, scope)?,
            Ast::InfixExpr(left, right, op) => {
                self.compile_infix_expression(&(*left), &(*right), op, _module, builder, scope)?
            }
            Ast::VarRef(v) => {
                let (var, var_type) = scope.as_ref().borrow().get(*(v).clone())?;
                (builder.use_var(var), var_type.clone())
            }

            _ => todo!("Unknown expression type"),
        };
        return Ok(to_ret)
    }
}