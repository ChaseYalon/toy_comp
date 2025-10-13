use crate::compiler::Compiler;
use std::io::{self, Write};

pub mod compiler;
mod lexer;
pub mod parser;
mod token;
#[macro_use]
mod macros;

use crate::{lexer::Lexer, parser::Parser};

fn main() {
    loop {
        let mut input = String::from("");
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut c = Compiler::new();
        print!(">");
        io::stdout().flush().unwrap();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        input = String::from(input.trim());
        if input == String::from("exit") {
            println!("Exiting");
            std::process::exit(0);
        }
        let user_fn = c.compile(p.parse(l.lex(String::from(input))));
        println!(">>{}", user_fn());
    }
}
