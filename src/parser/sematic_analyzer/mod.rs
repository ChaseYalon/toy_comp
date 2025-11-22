use crate::parser::Ast;
use crate::errors::ToyError;

#[derive(Debug)]
pub struct SematicAnalyzer {
}
//the goal of this module is to extract type checking logic from the boxer, ast_generator, and compiler
//this is to avoid huge amounts of duplicate code, making it difficult to refactor, epically surrounding struct interfaces
//plan to implement
// 1. WITHOUT using typing added to the AST by the ast_generator "confirm" that that typing is correct (it already is)
// 2. Ast_gen will be hardest so do it first
// 3. Because we have not depended on type hints from the ast_generator now we can rip out typechecking logic from the compiler
// 4. Now the boxer, should be easy
impl SematicAnalyzer {
    pub fn new() -> SematicAnalyzer {
        return SematicAnalyzer {
        }
    }
    pub fn analyze(&mut self, ast: &Vec<Ast>) -> Result<Vec<Ast>, ToyError> {
        return Ok(ast.clone());
    }
}

#[cfg(test)]
mod tests;