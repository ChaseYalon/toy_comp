use crate::errors::ToyError;
use crate::parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer, toy_box::TBox};
use crate::token::SpannedToken;

pub mod ast;
pub mod ast_gen;
pub mod boxer;
pub mod toy_box;
///Wrapper struct around boxer and generator sub modules
pub struct Parser {
    boxer: Boxer,
    ast_gen: AstGenerator,
}

impl Parser {
    pub fn new() -> Parser {
        return Parser {
            boxer: Boxer::new(),
            ast_gen: AstGenerator::new(),
        };
    }

    /// Create a Parser with a module prefix for name mangling standard library functions.
    /// The prefix should be in the form "std::<filename>" (e.g., "std::math")
    pub fn with_module_prefix(prefix: String) -> Parser {
        return Parser {
            boxer: Boxer::with_module_prefix(prefix.clone()),
            ast_gen: AstGenerator::with_module_prefix(prefix),
        };
    }

    pub fn parse(&mut self, input: Vec<SpannedToken>) -> Result<Vec<Ast>, ToyError> {
        let boxes = self.boxer.box_toks(input)?;
        let ast = self.ast_gen.generate(boxes)?;

        return Ok(ast);
    }

    //this is terrible
    //It totaly inverts the simple pipeline because ast_gen calls boxer
    //that said it is the best because it helps with lambdas and reduces logic duplications
    pub fn box_stmts(toks: Vec<SpannedToken>) -> Result<Vec<TBox>, ToyError> {
        Boxer::new().box_toks(toks)
    }
}
