mod ctla;
mod tir;
mod llvm;
use crate::parser::ast::Ast;
use crate::{codegen::ctla::CTLA, errors::ToyError};
use crate::codegen::llvm::LlvmGenerator;
use inkwell::context::Context;
use inkwell::module::Module;
use tir::AstToIrConverter;
pub use tir::ir::{Block, Function, SSAValue, TIR, TirType};

pub struct Generator<'a> {
    converter: AstToIrConverter,
    analyzer: CTLA,
    generator: LlvmGenerator<'a>
}

impl<'a> Generator<'a> {
    pub fn new(ctx: &'a Context, main_module: Module<'a>) -> Generator<'a> {
        Generator {
            converter: AstToIrConverter::new(),
            analyzer: CTLA::new(),
            generator: LlvmGenerator::new(ctx, main_module)//I hate that that is nesscary
        }
    }

    pub fn generate(&mut self, ast: Vec<Ast>, name: String) -> Result<(), ToyError>{
        let _ = self.converter.convert(ast)?;
        let ir = self.analyzer.analyze(self.converter.builder.clone())?;
        self.generator.generate(ir, name)?;
        return Ok(());
    }

}
