use crate::debug;
use crate::{compiler::vars::Scope, parser::ast::Ast, token::TypeTok};
use cranelift::prelude::*;
use cranelift_codegen::Context;
use cranelift_jit::JITModule;
use cranelift_module::FuncId;
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

mod exprs;
mod internals;
mod stmts;
mod vars;

pub enum OutputType {
    Jit(JITModule),
    Aot(ObjectModule),
}
pub static STUB_C: &str = include_str!("../c/stub.c");
pub static BUILTIN_C: &str = include_str!("../c/builtins.c");
pub struct Compiler {
    ast: Vec<Ast>,
    var_count: usize,
    main_scope: Rc<RefCell<Scope>>,
    funcs: HashMap<String, (TypeTok, FuncId)>,
    func_ir: Vec<String>,
    loop_cond_block: Option<Block>,
    loop_merge_block: Option<Block>,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            ast: Vec::new(),
            var_count: 0,
            main_scope: Rc::new(RefCell::new(Scope {
                vars: HashMap::new(),
                parent: None,
            })),
            funcs: HashMap::new(),
            func_ir: Vec::new(),
            loop_cond_block: None,
            loop_merge_block: None,
        }
    }

    fn compile_internal<M: Module>(&mut self, module: &mut M, ast: Vec<Ast>) -> (FuncId, Context) {
        let mut ctx: Context = module.make_context();

        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(types::I64));
        ctx.func.signature = sig;
        let mut builder_ctx = FunctionBuilderContext::new();
        let mut func_builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

        let main_block = func_builder.create_block();
        func_builder.switch_to_block(main_block);
        func_builder.append_block_params_for_function_params(main_block);
        func_builder.seal_block(main_block);

        let mut last_val: Option<(Value, TypeTok)> = None;

        self.declare_builtin_funcs(module);

        let sudo_main_scope = self.main_scope.clone();
        for node in ast {
            last_val = self.compile_stmt(node, module, &mut func_builder, &sudo_main_scope);
        }

        debug!(targets: ["compiler_verbose"], format!("Last val: {:?}", last_val));
        let (ret_val, _) =
            last_val.unwrap_or_else(|| (func_builder.ins().iconst(types::I64, 0), TypeTok::Int));
        func_builder.ins().return_(&[ret_val]);

        func_builder.finalize();

        let args: Vec<String> = env::args().collect();

        let func_id = module
            .declare_function("user_main", Linkage::Export, &ctx.func.signature)
            .unwrap();

        module.define_function(func_id, &mut ctx).unwrap();
        if args.contains(&"--save-ir".to_string()) {
            let str = format!("{}", ctx.func.display());
            self.func_ir.push(str);
            let mut ir: String = String::new();
            for s in self.func_ir.clone() {
                ir += &s;
            }
            let mut file = File::create("ir.clif").unwrap();
            file.write_all(ir.as_bytes()).unwrap();
        }
        module.clear_context(&mut ctx);

        (func_id, ctx)
    }

    pub fn compile(
        &mut self,
        ast: Vec<Ast>,
        should_jit: bool,
        path: Option<&str>,
    ) -> Option<fn() -> i64> {
        if !should_jit {
            let o_path = path.unwrap_or("program.exe");

            let base_name = Path::new(o_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("program");
            let obj_temp = format!("{}.obj", base_name);
            let stub_temp = format!("{}_stub.c", base_name);
            let builtin_temp = format!("{}_builtins.c", base_name);
            let obj_path = Path::new(&obj_temp);
            let stub_path = Path::new(&stub_temp);
            let builtin_path = Path::new(&builtin_temp);

            let mut obj_file = File::create(obj_path).unwrap();
            obj_file
                .write_all(&self.compile_to_object(ast.clone()))
                .unwrap();

            let mut stub_file = File::create(&stub_path).unwrap();
            stub_file.write_all(STUB_C.as_bytes()).unwrap();

            let mut builtin_file = File::create(builtin_path).unwrap();
            builtin_file.write_all(BUILTIN_C.as_bytes()).unwrap();

            let status = Command::new("gcc")
                .args(&[
                    obj_path.to_str().unwrap(),
                    stub_path.to_str().unwrap(),
                    builtin_path.to_str().unwrap(),
                    "-o",
                    o_path,
                ])
                .status()
                .expect("failed to execute gcc");

            if !status.success() {
                panic!("GCC failed with exit code {:?}", status.code());
            }
            //remove c objs
            let _ = std::fs::remove_file(stub_path);
            let _ = std::fs::remove_file(builtin_path);
            let args: Vec<String> = env::args().collect();
            if !args.contains(&"--save-temp".to_string()) {
                let _ = std::fs::remove_file(obj_path);
            }

            return None;
        }
        self.ast = ast.clone();
        let mut module = self.make_jit();

        let (func_id, _ctx) = self.compile_internal(&mut module, ast);

        module.finalize_definitions().unwrap();

        let code_ptr = module.get_finalized_function(func_id);
        Some(unsafe { std::mem::transmute::<_, fn() -> i64>(code_ptr) })
    }
}

#[cfg(test)]
mod tests;
