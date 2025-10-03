use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, FuncId, Linkage};
use cranelift::prelude::*;
use crate::parser::ast::{Ast, InfixOp};


struct Compiler{
    ast: Vec<Ast>,
}

impl Compiler{
    pub fn new() -> Compiler{
        let a_vec: Vec<Ast> = Vec::new();
        return Compiler { 
            ast: a_vec
        }
    }
    fn make_jit() -> JITModule {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names());
        return JITModule::new(builder.unwrap())
    }
    fn compile_expr(expr: &Ast, module: &mut JITModule) -> *const u8 {
        if expr.node_type() != "IntLit" || expr.node_type() != "InfixExpr"{
            panic!("[ERROR] Unknown value {} of type {}", expr, expr.node_type());
        }
        let mut ctx = module.make_context();
    }
}