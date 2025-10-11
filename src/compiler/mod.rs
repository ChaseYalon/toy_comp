use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, Linkage};
use cranelift::prelude::*;
use crate::parser::ast::{Ast, InfixOp};

pub struct Compiler {
    ast: Vec<Ast>,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler { ast: Vec::new() }
    }

    fn make_jit(&self) -> JITModule {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names());
        JITModule::new(builder.unwrap())
    }

    fn compile_expr(&self, expr: &Ast, _module: &mut JITModule, builder: &mut FunctionBuilder<'_>) -> Value {
        if expr.node_type() != "IntLit" && expr.node_type() != "InfixExpr" {
            panic!("[ERROR] Unknown AST node type: {}", expr.node_type());
        }

        match expr {
            Ast::IntLit(n) => builder.ins().iconst(types::I64, *n),
            Ast::InfixExpr(left, right, op) => {
                let l = self.compile_expr(left, _module, builder);
                let r = self.compile_expr(right, _module, builder);
                match op {
                    InfixOp::Plus => builder.ins().iadd(l, r),
                    InfixOp::Minus => builder.ins().isub(l, r),
                    InfixOp::Multiply => builder.ins().imul(l, r),
                    InfixOp::Divide => builder.ins().sdiv(l, r),
                }
            }
        }
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
        for node in ast {
            if node.node_type() == "IntLit" || node.node_type() == "InfixExpr" {
                last_val = Some(self.compile_expr(&node, &mut module, &mut func_builder));
            }
        }

        let ret_val = last_val.unwrap_or_else(|| func_builder.ins().iconst(types::I64, 0));
        func_builder.ins().return_(&[ret_val]);

        func_builder.seal_all_blocks();
        func_builder.finalize();

        let func_id = module
            .declare_function("main", Linkage::Export, &ctx.func.signature)
            .unwrap();

        module.define_function(func_id, &mut ctx).unwrap();
        module.clear_context(&mut ctx);
        module.finalize_definitions();

        let code_ptr = module.get_finalized_function(func_id);
        unsafe { std::mem::transmute::<_, fn() -> i64>(code_ptr) }
    }
}

#[cfg(test)]
mod tests;