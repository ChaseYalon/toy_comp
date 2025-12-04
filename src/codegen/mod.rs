mod tir;

use crate::errors::ToyError;
use crate::parser::ast::Ast;
use tir::AstToIrConverter;
pub use tir::ir::{Block, Function, SSAValue, TIR, TirType};

pub struct Generator {
    converter: AstToIrConverter,
}

impl Generator {
    pub fn new() -> Generator {
        Generator {
            converter: AstToIrConverter::new(),
        }
    }

    pub fn generate(&mut self, ast: Vec<Ast>) -> Result<Vec<Function>, ToyError> {
        self.converter.convert(ast)
    }
}
