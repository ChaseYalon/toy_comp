pub mod ctla;
mod llvm;
mod tir;
use crate::codegen::llvm::LlvmGenerator;
use crate::errors::{Span, ToyErrorType};
use crate::parser::ast::Ast;
use crate::token::TypeTok;
use crate::{codegen::ctla::CTLA, errors::ToyError};
use ctla::cfg::CFGFunction;
use inkwell::context::Context;
use inkwell::module::Module;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use tir::AstToIrConverter;
pub use tir::ir::{Block, Function, SSAValue, TIR, TirType};
pub struct Generator<'a> {
    converter: AstToIrConverter,
    analyzer: CTLA,
    generator: LlvmGenerator<'a>,
}

impl<'a> Generator<'a> {
    pub fn new(ctx: &'a Context, main_module: Module<'a>) -> Generator<'a> {
        Generator {
            converter: AstToIrConverter::new(),
            analyzer: CTLA::new(),
            generator: LlvmGenerator::new(ctx, main_module), //I hate that that is nesscary
        }
    }

    pub fn register_imported_func(&mut self, name: String, ret_type: TypeTok) {
        self.converter
            .builder
            .register_extern_func(name, ret_type, false);
    }
    pub fn set_original_text(&mut self, text: String) {
        self.analyzer.set_original_text(text);
    }

    pub fn set_external_modules(&mut self, modules: HashMap<String, Vec<ctla::FunctionSummary>>) {
        self.analyzer.set_external_modules(modules);
    }

    fn pretty_print_tir(ir: &Vec<Function>) -> Result<String, ToyError> {
        let res = serde_json::to_string(ir);
        match res {
            Ok(s) => return Ok(s),
            Err(_) => {
                return Err(ToyError::new(
                    ToyErrorType::SerializationError,
                    Span::null_span(),
                ));
            }
        };
    }
    fn pretty_print_cfg(cfg: &Vec<CFGFunction>) -> Result<String, ToyError> {
        let res = serde_json::to_string(cfg);
        match res {
            Ok(s) => Ok(s),
            Err(_) => Err(ToyError::new(
                ToyErrorType::SerializationError,
                Span::null_span(),
            )),
        }
    }
    pub fn compile_to_object(
        &mut self,
        ast: Vec<Ast>,
        name: String,
        is_main: bool,
    ) -> Result<(), ToyError> {
        let pre_ctla_ir = self.converter.convert(ast, is_main, &name)?;
        let args: Vec<String> = env::args().collect();
        if args.contains(&"--debug-tir".to_string()) || args.contains(&"--debug-ALL".to_string()) {
            let s = Generator::pretty_print_tir(&pre_ctla_ir)?;
            fs::write("./debug/TIR.json", s).unwrap(); //rly should be an io error -> toy error conversion
        }
        let ir = self.analyzer.analyze(self.converter.builder.clone())?;
        if args.contains(&"--debug-cfg".to_string()) || args.contains(&"--debug-ALL".to_string()) {
            let s = Generator::pretty_print_cfg(self.analyzer.cfg_functions())?;
            fs::write("./debug/CFG.json", s).unwrap(); //rly should be an io error -> toy error conversion
        }
        self.generator.generate(ir, name)?;
        Ok(())
    }
}
