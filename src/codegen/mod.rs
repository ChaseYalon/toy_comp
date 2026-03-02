mod ctla;
mod llvm;
mod tir;
use crate::codegen::llvm::LlvmGenerator;
use crate::parser::ast::Ast;
use crate::token::TypeTok;
use crate::{codegen::ctla::CTLA, errors::ToyError};
use inkwell::context::Context;
use inkwell::module::Module;
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

    pub fn compile_to_object(
        &mut self,
        ast: Vec<Ast>,
        name: String,
        is_main: bool,
    ) -> Result<(), ToyError> {
        let _ = self.converter.convert(ast, is_main, &name)?;
        let ir = self.analyzer.analyze(self.converter.builder.clone())?;
        self.generator.generate(ir, name)?;
        Ok(())
    }
}
