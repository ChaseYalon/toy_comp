#![feature(error_generic_member_access)]
#![feature(backtrace_frames)]

use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
mod lexer;
pub mod parser;
mod token;
#[macro_use]
mod macros;
pub mod codegen;
mod errors;
mod ffi;
use crate::{lexer::Lexer, parser::Parser};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--repl".to_string()) {
        loop {
            let mut input = String::from("");
            let mut l = Lexer::new();
            let mut p = Parser::new();
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
            let args: Vec<String> = env::args().collect();
            let should_jit = args.contains(&"--aot".to_string());

            // Do this:
            let result = match p.parse(l.lex(String::from(input)).unwrap()) {
                Ok(ast) => ast,
                Err(e) => {
                    eprintln!("{}", e); // This will use your Display implementation
                    std::process::exit(1);
                }
            };
        }
    }
    let mut filename: &String = &"NULL".to_string();
    if args.len() > 1 {
        filename = &args[1];
    }

    let contents = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading {}: {}", filename, err);
        process::exit(1);
    });
    let mut l = Lexer::new();
    let mut p = Parser::new();
    let should_jit = if args.contains(&"--aot".to_string()) {
        false
    } else {
        true
    };
    let path = if !should_jit {
        Some("output.exe")
    } else {
        None
    };
}
