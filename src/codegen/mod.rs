mod tir;
mod ctla;
use crate::{codegen::ctla::CTLA, errors::ToyError};
use crate::parser::ast::Ast;
use tir::AstToIrConverter;
pub use tir::ir::{Block, Function, SSAValue, TIR, TirType};

pub struct Generator {
    converter: AstToIrConverter,
    analyzer: CTLA
}

impl Generator {
    pub fn new() -> Generator {
        Generator {
            converter: AstToIrConverter::new(),
            analyzer: CTLA::new()
        }
    }

    pub fn generate(&mut self, ast: Vec<Ast>) -> Result<Vec<Function>, ToyError> {
        self.analyzer.analyze(self.converter.convert(ast)?)
    }
}
