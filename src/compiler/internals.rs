use super::Compiler;
use crate::parser::ast::Ast;
use crate::token::TypeTok;

use crate::ffi::*;
use cranelift::prelude::*;
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;
use crate::errors::ToyError;

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
        jit_builder.symbol("toy_type_to_bool", toy_type_to_bool as *const u8);
        jit_builder.symbol("toy_type_to_int", toy_type_to_int as *const u8);
        jit_builder.symbol("toy_int_to_float", toy_int_to_float as *const u8);
        jit_builder.symbol(
            "toy_float_bits_to_double",
            toy_float_bits_to_double as *const u8,
        );
        jit_builder.symbol(
            "toy_double_to_float_bits",
            toy_double_to_float_bits as *const u8,
        );
        jit_builder.symbol("toy_type_to_float", toy_type_to_float as *const u8);
        jit_builder.symbol("toy_write_to_arr", toy_write_to_arr as *const u8);
        jit_builder.symbol("toy_read_from_arr", toy_read_from_arr as *const u8);
        jit_builder.symbol("toy_malloc_arr", toy_malloc_arr as *const u8);
        jit_builder.symbol("toy_arrlen", toy_arrlen as *const u8);
        jit_builder.symbol("toy_put", toy_put as *const u8);
        jit_builder.symbol("toy_get", toy_get as *const u8);
        jit_builder.symbol("toy_create_map", toy_create_map as *const u8);
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
        self.funcs.insert(
            "malloc".to_string(),
            (TypeTok::Int, func, vec!["size".to_string()]),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_print", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "print".to_string(),
            (
                TypeTok::Void,
                func,
                vec![
                    "value".to_string(),
                    "type".to_string(),
                    "dimension".to_string(),
                ],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_println", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "println".to_string(),
            (
                TypeTok::Void,
                func,
                vec![
                    "value".to_string(),
                    "type".to_string(),
                    "dimension".to_string(),
                ],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); //str a
        sig.params.push(AbiParam::new(types::I64)); //str b
        sig.returns.push(AbiParam::new(types::I64)); //Ptr to a + b
        let func = module
            .declare_function("toy_concat", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "concat".to_string(),
            (
                TypeTok::Int,
                func,
                vec!["str1".to_string(), "str2".to_string()],
            ),
        ); //Returns a pointer to the new string

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_strequal", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "strequal".to_string(),
            (
                TypeTok::Bool,
                func,
                vec!["str1".to_string(), "str2".to_string()],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_strlen", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "strlen".to_string(),
            (TypeTok::Int, func, vec!["str".to_string()]),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_type_to_str", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "str".to_string(),
            (
                TypeTok::Str,
                func,
                vec!["convertible_to_string".to_string()],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_type_to_bool", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "bool".to_string(),
            (TypeTok::Bool, func, vec!["convertible_to_bool".to_string()]),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_type_to_int", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "int".to_string(),
            (TypeTok::Int, func, vec!["convertible_to_int".to_string()]),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::F64));
        let func = module
            .declare_function("toy_int_to_float", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_int_to_float".to_string(),
            (
                TypeTok::Float,
                func,
                vec!["convertible_to_float".to_string()],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::F64));
        let func = module
            .declare_function("toy_float_bits_to_double", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_float_bits_to_double".to_string(),
            (TypeTok::Float, func, vec!["float_literal_bits".to_string()]),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::F64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_double_to_float_bits", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_double_to_float_bits".to_string(),
            (TypeTok::Int, func, vec!["float_literal".to_string()]),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_type_to_float", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "float".to_string(),
            (
                TypeTok::Float,
                func,
                vec!["convertible_to_float".to_string()],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_write_to_arr", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_write_to_arr".to_string(),
            (
                TypeTok::Void,
                func,
                vec![
                    "arr_ptr".to_string(),
                    "value".to_string(),
                    "idx".to_string(),
                    "type".to_string(),
                ],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_read_from_arr", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_read_from_arr".to_string(),
            (
                TypeTok::Any,
                func,
                vec!["arr_ptr".to_string(), "idx".to_string()],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_malloc_arr", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_malloc_arr".to_string(),
            (
                TypeTok::Any,
                func,
                vec!["len".to_string(), "type".to_string()],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_arrlen", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "arrlen".to_string(),
            (TypeTok::Int, func, vec!["arr_ptr".to_string()]),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_put", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_put".to_string(),
            (
                TypeTok::Void,
                func,
                vec![
                    "hashmap_pointer".to_string(),
                    "key".to_string(),
                    "value".to_string(),
                ],
            ),
        );

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_get", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert(
            "toy_get".to_string(),
            (
                TypeTok::Int,
                func,
                vec!["hashmap_pointer".to_string(), "key".to_string()],
            ),
        );

        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_create_map", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("toy_create_map".to_string(), (TypeTok::Void, func, vec![]));
    }

    pub fn compile_to_object(&mut self, ast: Vec<Ast>) -> Result<Vec<u8>, ToyError> {
        self.ast = ast.clone();
        let mut module = self.make_object();

        let (_func_id, _ctx) = self.compile_internal(&mut module, ast)?;

        let object_product = module.finish();
        Ok(object_product.emit().unwrap())
    }
}
