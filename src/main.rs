mod lexer;
mod token;
#[macro_use]
mod macros;

use crate::lexer::Lexer;

fn main() {
    let mut l = Lexer::new();
    l.lex(String::from(""));
    println!("Hello, world!");
}
