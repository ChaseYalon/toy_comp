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
pub static FILE_EXTENSION_O: &str = if cfg!(target_os = "windows") {
    "obj"
} else if cfg!(target_os = "linux") {
    "o"
} else {
    panic!("[ERROR] Only supported OS's are windows and linux")
};

pub static FILE_EXTENSION_EXE: &str = if cfg!(target_os = "windows") {
    ".exe"
} else if cfg!(target_os = "linux") {
    ""
} else {
    panic!("[ERROR] Only supported OS's are windows and linux")
};

pub static STUB_C: &str = include_str!("../c/stub.c");
pub static BUILTIN_C: &str = include_str!("../c/builtins.c");
pub static BUILTIN_H: &str = include_str!("../c/builtins.h");
pub struct Compiler {
    ast: Vec<Ast>,
    var_count: usize,
    main_scope: Rc<RefCell<Scope>>,
    funcs: HashMap<String, (TypeTok, FuncId, Vec<String>)>,
    func_ir: Vec<String>,
    loop_cond_block: Option<Block>,
    loop_merge_block: Option<Block>,
    current_struct_name: Option<String>, //this code is awful
    is_in_func: bool, //I hate this but it is easier then refactoring to use Result<T, E>
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            ast: Vec::new(),
            var_count: 0,
            main_scope: Rc::new(RefCell::new(Scope {
                vars: HashMap::new(),
                parent: None,
                interfaces: HashMap::new(),
                structs: HashMap::new(),
                unresolved_structs: HashMap::new()
            })),
            funcs: HashMap::new(),
            func_ir: Vec::new(),
            loop_cond_block: None,
            loop_merge_block: None,
            current_struct_name: None,
            is_in_func: false,
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
            let exe_val = format!("program{}", FILE_EXTENSION_EXE);
            let output_path = path.unwrap_or(&exe_val);
            let base_name = Path::new(output_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("program");
            let temp_obj_name = format!("{}.{}", base_name, FILE_EXTENSION_O);
            let obj_path = Path::new("temp").join(temp_obj_name);
            let mut obj_file = File::create(obj_path.clone()).unwrap();
            let target = env!("TARGET").replace("\"", "");
            let lib_str = format!("lib/{}/", target);
            let lib_path = Path::new(&lib_str);
            obj_file
                .write_all(&self.compile_to_object(ast.clone()))
                .unwrap();
            //fuck the borrow checker
            let crt2_path = lib_path.join("crt2.o");
            let crtbegin_path = lib_path.join("crtbegin.o");
            let crt1_path = lib_path.join("crt1.o");
            let crti_path = lib_path.join("crti.o");
            let lbruntime_path = lib_path.join("libruntime.a");
            let crtn_path = lib_path.join("crtn.o");
            let libc_path = lib_path.join("libc.so.6");
            let libm_path = lib_path.join("libm.so.6");
            let args: Vec<&str> = if env::consts::OS == "windows" {
                vec![
                    "-m",
                    "i386pep",
                    crt2_path.to_str().unwrap(),
                    crtbegin_path.to_str().unwrap(),
                    obj_path.to_str().unwrap(),
                    "-L",
                    lib_path.to_str().unwrap(),
                    "-lruntime",
                    "-lmingw32",
                    "-lmingwex",
                    "-lmsvcrt",
                    "-lkernel32",
                    "-luser32",
                    "-lshell32",
                    "-lgcc",
                    "-o",
                    path.unwrap(),
                ]
            } else {
                vec![
                    "-m",
                    "elf_x86_64",
                    "-o",
                    path.unwrap(),
                    crt1_path.to_str().unwrap(),
                    crti_path.to_str().unwrap(),
                    obj_path.to_str().unwrap(),
                    lbruntime_path.to_str().unwrap(),
                    crtn_path.to_str().unwrap(),
                    libc_path.to_str().unwrap(),
                    libm_path.to_str().unwrap(),
                    "-dynamic-linker",
                    "/lib64/ld-linux-x86-64.so.2",
                ]
            };

            let status = Command::new(lib_path.join("ld.lld"))
                .args(args)
                .status()
                .expect("Failed to link");

            if !status.success() {
                panic!(
                    "[ERROR] ld failed with exit code {:?}, Error: {:?}",
                    status.code(),
                    status
                );
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
