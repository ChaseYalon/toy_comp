use crate::{codegen::{Function, tir::viewer::Viewer}, errors::ToyError};



pub struct CTLA {
    viewer: Viewer
}

impl CTLA {
    pub fn new() -> CTLA {
        return CTLA { 
            viewer: Viewer::new()
        }
    }
    pub fn analyze(&mut self, funcs: Vec<Function>) -> Result<Vec<Function>, ToyError> {
        self.viewer.set_funcs(funcs);
        return Ok(self.viewer.funcs());
    }
}

#[cfg(test)]
mod tests;