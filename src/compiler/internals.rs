use super::Compiler;
use crate::token::TypeTok;
use crate::parser::ast::Ast;

use cranelift::prelude::*;
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;
unsafe extern "C" {
    unsafe fn toy_print(input: i64, datatype: i64);
    unsafe fn toy_println(input: i64, datatype: i64);
    unsafe fn toy_malloc(ptr: i64) -> i64;
    unsafe fn toy_concat(sp1: i64, sp2: i64) -> i64;
    unsafe fn toy_strequal(sp1: i64, sp2: i64) -> i64;
    unsafe fn toy_strlen(sp1: i64) -> i64;
    unsafe fn toy_type_to_str(ptr: i64, ptr_type: i64) -> i64;
}

impl Compiler {
    pub fn make_jit(&self) -> JITModule {
        let mut jit_builder = JITBuilder::new(default_libcall_names()).unwrap();
        jit_builder.symbol("toy_print", toy_print as *const u8);
        jit_builder.symbol("toy_println", toy_println as *const u8);
        jit_builder.symbol("toy_malloc", toy_malloc as *const u8);
        jit_builder.symbol("toy_concat", toy_concat as *const u8);
        jit_builder.symbol("toy_strequal", toy_strequal as *const u8);
        jit_builder.symbol("toy_strlen", toy_strlen as *const u8);
        jit_builder.symbol("toy_type_to_str", toy_type_to_str as *const u8);
        JITModule::new(jit_builder)
    }

    pub fn make_object(&self) -> ObjectModule {
        let triple = Triple::host();
        let isa_builder = isa::lookup(triple).expect("ISA lookup failed");

        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "true").unwrap();
        let flags = settings::Flags::new(flag_builder);

        let isa = isa_builder.finish(flags).expect("Failed to finish ISA");

        let obj_builder = ObjectBuilder::new(isa, "toy_lang".to_string(), default_libcall_names())
            .expect("ObjectBuilder creation failed");
        ObjectModule::new(obj_builder)
    }
    pub fn declare_builtin_funcs<M: Module>(&mut self, module: &mut M) {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        //Toy malloc takes a pointer to a string and allocates it in memory, returning the pointer to that allocation
        let func = module
            .declare_function("toy_malloc", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("malloc".to_string(), (TypeTok::Int, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_print", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("print".to_string(), (TypeTok::Void, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_println", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("println".to_string(), (TypeTok::Void, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); //str a
        sig.params.push(AbiParam::new(types::I64)); //str b
        sig.returns.push(AbiParam::new(types::I64)); //Ptr to a + b
        let func = module
            .declare_function("toy_concat", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("concat".to_string(), (TypeTok::Int, func)); //Returns a pointer to the new string

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_strequal", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("strequal".to_string(), (TypeTok::Bool, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_strlen", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert("len".to_string(), (TypeTok::Int, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module.declare_function("toy_type_to_str", Linkage::Import, &sig).unwrap();
        self.funcs.insert("str".to_string(), (TypeTok::Str, func));
    }

    pub fn compile_to_object(&mut self, ast: Vec<Ast>) -> Vec<u8> {
        self.ast = ast.clone();
        let mut module = self.make_object();

        let (_func_id, _ctx) = self.compile_internal(&mut module, ast);

        let object_product = module.finish();
        object_product.emit().unwrap()
    }
}