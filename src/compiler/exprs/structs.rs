use super::super::Compiler;
use super::Scope;
use crate::parser::ast::Ast;
use crate::token::TypeTok;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cranelift::prelude::*;
use cranelift_module::Module;

impl Compiler {
    pub fn compile_struct_lit<M: Module>(
        &mut self,
        struct_name: String,
        value_map: &HashMap<String, (Ast, TypeTok)>,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> (Value, TypeTok) {
        let (_, global_hashmap_put, _) = self.funcs.get("toy_put").unwrap();
        let (_, global_create_map, _) = self.funcs.get("toy_create_map").unwrap();
        let toy_put = module.declare_func_in_func(global_hashmap_put.clone(), &mut builder.func);
        let toy_create_map = module.declare_func_in_func(global_create_map.clone(), builder.func);
        let interface_name = struct_name.clone();

        let interface_types = scope.borrow_mut().get_interface(interface_name.clone());
        let create_res = builder.ins().call(toy_create_map, &[]);
        let map_ptr_val = builder.inst_results(create_res)[0];
        let map_ptr_idx = self.var_count;
        self.var_count += 1;
        builder.declare_var(Variable::new(map_ptr_idx), types::I64);
        builder.def_var(Variable::new(map_ptr_idx), map_ptr_val);
        for (key, (value, _)) in value_map.iter() {
            let (k, _) = self.compile_expr(
                &Ast::StringLit(Box::new(key.clone())),
                module,
                builder,
                scope,
            );
            let (v, _) = self.compile_expr(value, module, builder, scope);
            let _ = builder.ins().call(toy_put, &[map_ptr_val, k, v]);
        }
        let boxed: HashMap<String, Box<TypeTok>> = interface_types
            .clone()
            .into_iter()
            .map(|(k, v)| (k, Box::new(v)))
            .collect();
        scope.borrow_mut().set_struct(
            self.current_struct_name.clone().unwrap(),
            interface_name,
            Variable::new(self.var_count),
        );
        //scope.borrow_mut().set_struct(name, val, ptr);
        (map_ptr_val, TypeTok::Struct(boxed))
    }
    pub fn compile_struct_ref<M: Module>(
        &mut self,
        struct_name: String,
        keys: &Vec<String>,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> (Value, TypeTok) {
        let (_, global_hashmap_get, _) = self.funcs.get("toy_get").unwrap();
        let toy_get = module.declare_func_in_func(global_hashmap_get.clone(), &mut builder.func);
        let name = struct_name.clone();
        let current_ptr_var; //temp
        if self.is_in_func {
            let (_, var) = scope.borrow().get_unresolved_struct(name.clone());
            current_ptr_var = var;
        } else {
            let (_, var) = scope.borrow().get_struct(name.clone());
            current_ptr_var = var;
        }
        let (_, mut current_type) = scope.borrow().get(name.clone());

        let mut current_pointer_val = builder.use_var(current_ptr_var);
        for key in keys.iter() {
            let (value_key, _) = self.compile_expr(
                &Ast::StringLit(Box::new(key.clone())),
                module,
                builder,
                scope,
            );

            let call_res = builder
                .ins()
                .call(toy_get, &[current_pointer_val, value_key]);
            let value = builder.inst_results(call_res)[0];

            let kv = match &current_type {
                TypeTok::Struct(kv) => kv,
                _ => panic!("[ERROR] Cannot access field '{}' on non-struct type", key),
            };

            current_type = *(kv.get(key).unwrap()).clone();
            current_pointer_val = value;
        }

        (current_pointer_val, current_type)
    }
}