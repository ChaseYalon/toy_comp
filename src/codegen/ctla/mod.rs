use crate::{
    codegen::{
        Function, SSAValue, TIR, TirType, tir::{ir::TirBuilder}
    },
    errors::ToyError,
};

pub struct CTLA {
    builder: TirBuilder
}

impl CTLA {
    pub fn new() -> CTLA {
        return CTLA {
            builder: TirBuilder::new()
        };
    }
    pub fn analyze(&mut self, builder: TirBuilder) -> Result<Vec<Function>, ToyError> {
        self.builder = builder;
        return Ok(self.builder.funcs.clone());
    }
}

#[cfg(test)]
mod tests;