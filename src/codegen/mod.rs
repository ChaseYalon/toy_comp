mod ctla;
mod tir;
mod llvm;
use crate::parser::ast::Ast;
use crate::{codegen::ctla::CTLA, errors::ToyError};
use crate::codegen::llvm::LlvmGenerator;
use inkwell::context::Context;
use tir::AstToIrConverter;
pub use tir::ir::{Block, Function, SSAValue, TIR, TirType};

pub struct Generator<'a> {
    converter: AstToIrConverter,
    analyzer: CTLA,
    generator: LlvmGenerator<'a>
}

impl<'a> Generator<'a> {
    pub fn new(ctx: &'a Context) -> Generator<'a> {
        Generator {
            converter: AstToIrConverter::new(),
            analyzer: CTLA::new(),
            generator: LlvmGenerator::new(ctx)//I hate that that is nesscary
        }
    }

    pub fn generate(&mut self, ast: Vec<Ast>) -> Result<(), ToyError>{
        let _ = self.converter.convert(ast)?;
        let ir = self.analyzer.analyze(self.converter.builder.clone())?;
        self.generator.generate(ir)?;
        return Ok(());
    }

    pub fn compile(&mut self, ast: Vec<Ast>, output_path: &str) -> Result<(), ToyError>{
        let _ = self.converter.convert(ast)?;
        let ir = self.analyzer.analyze(self.converter.builder.clone())?;
        self.generator.generate(ir)?;
        return Ok(());
    }
}
