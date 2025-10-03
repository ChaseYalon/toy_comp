mod lexer;
pub mod parser;
mod token;
pub mod compiler;
#[macro_use]
mod macros;

use crate::{lexer::Lexer, parser::Parser};

fn main() {
    let mut l = Lexer::new();
    let mut p = Parser::new();
    p.parse(l.lex(String::from("")));

    println!("Hello, world!");
}
