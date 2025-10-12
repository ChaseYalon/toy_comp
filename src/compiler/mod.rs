use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, Linkage};
use cranelift::prelude::*;
use crate::{parser::ast::{Ast, InfixOp}, token::TypeTok};
use std::{collections::HashMap};
use crate::debug;
pub struct Compiler {
    ast: Vec<Ast>,
    var_count: usize,
    main_scope: Scope
}
#[derive(Debug, Clone, Default)]
struct Scope{
    vars: HashMap<String, (Variable, TypeTok)>
}
impl Compiler {
    pub fn new() -> Compiler {
        Compiler { 
            ast: Vec::new(), 
            var_count: 0,
            main_scope: Scope { 
                vars: HashMap::new() 
            }
        }
    }

    fn make_jit(&self) -> JITModule {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names());
        JITModule::new(builder.unwrap())
    }

    fn compile_expr(&self, expr: &Ast, _module: &mut JITModule, builder: &mut FunctionBuilder<'_>, scope: &Scope) -> (Value, TypeTok) {
        debug!("in compile expression");
        if expr.node_type() != "IntLit" && expr.node_type() != "InfixExpr" && expr.node_type() != "VarRef" && expr.node_type() != "BoolLit" {
            panic!("[ERROR] Unknown AST node type: {}", expr.node_type());
        }

        match expr {
            Ast::IntLit(n) => (builder.ins().iconst(types::I64, *n), TypeTok::Int),
            Ast::BoolLit(b) => {
                let is_true: i64 = if *b { 1 } else { 0 };
                (builder.ins().iconst(types::I64, is_true), TypeTok::Bool)
            },
            Ast::InfixExpr(left, right, op) => {
                let (l, l_t) = self.compile_expr(left, _module, builder, scope);
                let (r, r_t) = self.compile_expr(right, _module, builder, scope);
                if l_t.type_str() == "Int" && r_t.type_str() == "Int" {
                    return match op {
                        InfixOp::Plus => (builder.ins().iadd(l, r), TypeTok::Int),
                        InfixOp::Minus => (builder.ins().isub(l, r), TypeTok::Int),
                        InfixOp::Multiply => (builder.ins().imul(l, r), TypeTok::Int),
                        InfixOp::Divide => (builder.ins().sdiv(l, r), TypeTok::Int),
                    }
                }
                panic!("[ERROR] Unknown type combination, got l: {}, r: {}", l, r);
            }
            Ast::VarRef(v) => {
                let v_string = v.clone();
                let (var, var_type) = scope.vars.get(&*v_string)
                    .unwrap_or_else(|| panic!("[ERROR] Undefined variable, got {}", v_string));
                debug!(var);
                (builder.use_var(*var), var_type.clone())
            }

            _ => todo!("Unknown expression type")
        }
    }
    fn compile_var_reassign(&mut self, var_res: &Ast, _module: &mut JITModule, builder: &mut FunctionBuilder<'_>, scope: &mut Scope){
        if var_res.node_type() != "VarReassign" {
            panic!("[ERROR] Expecting VarReassign, got {}", var_res);
        }
        let var_name: String;
        let new_val: Ast;
        match var_res {
            Ast::VarReassign(name, new_val_b) => {var_name = *name.clone(); new_val = *new_val_b.clone()},
            _ => panic!("[ERROR] Expecting VarReassign, got {}", var_res)
        }
        let (old_val, old_type) = scope.vars.get(&var_name).unwrap();
        let (val, val_type) = self.compile_expr(&new_val, _module, builder, scope);
        let var = Variable::new(self.var_count);
        self.var_count += 1;
        builder.declare_var(var, types::I64);
        builder.def_var(var, val);
        scope.vars.insert(var_name, (var, old_type.clone()));

    }
    fn compile_var_dec(&mut self, var_dec: &Ast, _module: &mut JITModule, builder: &mut FunctionBuilder<'_>, scope: &mut Scope){
        if var_dec.node_type() != "VarDec"{
            panic!("[ERROR] Expected variable declarations, got {}", var_dec);
        }
        let name: String;
        let val: Ast;
        let t_o: &TypeTok;
        match var_dec{
            Ast::VarDec(n,t, v) => {
                name = *n.clone();
                val = *v.clone();
                t_o = t;
            }
            _ => {
                panic!("[ERROR] Expected variable declarations, got {}", var_dec);
            }
        }
        let (val, val_type) = self.compile_expr(&val, _module, builder, scope);
        let var = Variable::new(self.var_count);
        builder.declare_var(var, types::I64);
        builder.def_var(var, val);    
        self.var_count += 1;
        debug!(val);
        debug!(var);
        scope.vars.insert(name, (var, t_o.clone()));

    }
    pub fn compile(&mut self, ast: Vec<Ast>) -> fn() -> i64 {
        self.ast = ast.clone();
        let mut module = self.make_jit();
        let mut ctx = module.make_context();

        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(types::I64));
        ctx.func.signature = sig;

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut func_builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

        let main_block = func_builder.create_block();
        func_builder.switch_to_block(main_block);
        func_builder.append_block_params_for_function_params(main_block);

        let mut last_val = None;
        let mut sudo_main_scope = std::mem::take(&mut self.main_scope);

        for node in ast {
            if node.node_type() == "IntLit" || node.node_type() == "InfixExpr" || node.node_type() == "VarRef" {
                last_val = Some(self.compile_expr(&node, &mut module, &mut func_builder, &mut sudo_main_scope));
            }
            if node.node_type() == "VarDec" {
                self.compile_var_dec(&node, &mut module, &mut func_builder, &mut sudo_main_scope);
            }
            if node.node_type() == "VarReassign" {
                self.compile_var_reassign(&node, &mut module, &mut func_builder, &mut sudo_main_scope);
            }

        }
        self.main_scope = sudo_main_scope;
        let (ret_val, ret_type) = last_val.unwrap_or_else(|| (func_builder.ins().iconst(types::I64, 0), TypeTok::Int));
        func_builder.ins().return_(&[ret_val]);

        func_builder.seal_all_blocks();
        func_builder.finalize();

        let func_id = module
            .declare_function("main", Linkage::Export, &ctx.func.signature)
            .unwrap();

        module.define_function(func_id, &mut ctx).unwrap();
        module.clear_context(&mut ctx);
        let _ = module.finalize_definitions();

        let code_ptr = module.get_finalized_function(func_id);
        unsafe { std::mem::transmute::<_, fn() -> i64>(code_ptr) }
    }
}

#[cfg(test)]
mod tests;