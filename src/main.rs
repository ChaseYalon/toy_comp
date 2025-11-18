use crate::compiler::Compiler;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
pub mod compiler;
mod lexer;
pub mod parser;
mod token;
#[macro_use]
mod macros;
mod ffi;
mod errors;
use crate::{lexer::Lexer, parser::Parser};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--repl".to_string()) {
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
            let args: Vec<String> = env::args().collect();
            let should_jit = args.contains(&"--aot".to_string());
            let user_fn = c.compile(
                p.parse(l.lex(String::from(input))),
                !should_jit,
                Some("output.exe"),
            );
            if user_fn.is_some() {
                println!(">>{}", user_fn.unwrap()());
            } else {
                continue;
            }
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
    let mut c = Compiler::new();
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
    let res = c.compile(p.parse(l.lex(contents)), should_jit, path);
    if res.is_some() {
        println!("User fn: {}", res.unwrap()());
    }
}
