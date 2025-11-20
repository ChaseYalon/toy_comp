use super::super::Compiler;
use super::Scope;
use crate::parser::ast::Ast;
use crate::token::TypeTok;

use std::cell::RefCell;
use std::rc::Rc;

use cranelift::prelude::*;
use cranelift_module::Module;
use crate::errors::ToyError;

impl Compiler {
    pub fn compile_arr_lit<M: Module>(
        &mut self,
        arr: &Vec<Ast>,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<Value, ToyError> {
        let mut arr_items: Vec<Value> = Vec::new();
        let mut arr_types: Vec<TypeTok> = Vec::new();
        for expr in arr {
            let (val, t) = self.compile_expr(expr, module, builder, scope)?;
            arr_items.push(val);
            arr_types.push(t);
        }
        let (_, arr_malloc_global, _) = self.funcs.get("toy_malloc_arr").unwrap();
        let (_, arr_write_global, _) = self.funcs.get("toy_write_to_arr").unwrap();
        let arr_malloc = module.declare_func_in_func(*arr_malloc_global, &mut builder.func);
        let arr_write = module.declare_func_in_func(*arr_write_global, &mut builder.func);

        //Calls toy malloc with the correct params
        let len = builder.ins().iconst(types::I64, arr_items.len() as i64);
        let mut params = [len].to_vec();
        self.inject_type_param(&arr_types[0], false, module, builder, &mut params)?;
        let call_res = builder.ins().call(arr_malloc, params.as_slice());
        let arr_ptr = builder.inst_results(call_res)[0];

        for (i, item) in arr_items.iter().enumerate() {
            let mut params = [arr_ptr, item.clone()].to_vec();
            let idx = builder.ins().iconst(types::I64, i as i64);
            params.push(idx);
            self.inject_type_param(&arr_types[i], false, module, builder, &mut params)?;
            builder.ins().call(arr_write, params.as_slice());
        }

        return Ok(arr_ptr);
    }
    pub fn compile_arr_ref<M: Module>(
        &mut self,
        array_name: String,
        indices: &Vec<Ast>,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<(Value, TypeTok), ToyError> {
        let (arr_var, mut arr_type) = scope.as_ref().borrow().get(array_name)?;
        let (_, arr_read_global, _) = self.funcs.get("toy_read_from_arr").unwrap();
        let arr_read = module.declare_func_in_func(*arr_read_global, &mut builder.func);

        let mut current_ptr = builder.use_var(arr_var);

        for (_, idx_expr) in indices.iter().enumerate() {
            let (idx_val, _) = self.compile_expr(idx_expr, module, builder, scope)?;

            let call_params = [current_ptr, idx_val].to_vec();
            let call_inst = builder.ins().call(arr_read, call_params.as_slice());
            current_ptr = builder.inst_results(call_inst)[0];

            arr_type = match arr_type {
                TypeTok::IntArr(n) if n > 1 => TypeTok::IntArr(n - 1),
                TypeTok::BoolArr(n) if n > 1 => TypeTok::BoolArr(n - 1),
                TypeTok::StrArr(n) if n > 1 => TypeTok::StrArr(n - 1),
                TypeTok::FloatArr(n) if n > 1 => TypeTok::FloatArr(n - 1),
                TypeTok::AnyArr(n) if n > 1 => TypeTok::AnyArr(n - 1),
                TypeTok::IntArr(1) => TypeTok::Int,
                TypeTok::BoolArr(1) => TypeTok::Bool,
                TypeTok::StrArr(1) => TypeTok::Str,
                TypeTok::FloatArr(1) => TypeTok::Float,
                TypeTok::AnyArr(1) => TypeTok::Any,

                _ => return Err(ToyError::ArrayTypeInvalid)
            };
        }

        Ok((current_ptr, arr_type))
    }
}