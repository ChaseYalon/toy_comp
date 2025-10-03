use crate::{
    parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer},
    token::Token,
};

pub mod ast;
mod ast_gen;
mod boxer;
mod toy_box;

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
    pub fn parse(&mut self, input: Vec<Token>) -> Vec<Ast> {
        return self.ast_gen.generate(self.boxer.box_toks(input));
    }
}
