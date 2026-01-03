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

use inkwell::context::Context;
use inkwell::module::Module;

use crate::codegen::Generator;
use crate::errors::ToyError;
use crate::lexer::Lexer;
use crate::parser::Parser;
pub struct Compiler<'a> {
    lexer: Lexer,
    parser: Parser,
    generator: Generator<'a>,
}
impl<'a> Compiler<'a> {
    pub fn new(ctx: &'a Context, module: Module<'a>) -> Compiler<'a> {
        Compiler::<'a> {
            lexer: Lexer::new(),
            parser: Parser::new(),
            generator: Generator::new(ctx, module),
        }
    }
    pub fn compile(&mut self, source: String) -> Result<(), ToyError> {
        let tokens = self.lexer.lex(source)?;
        let ast = self.parser.parse(tokens)?;
        self.generator.generate(ast, "program".to_string())?;
        Ok(())
    }
}
fn run_repl() {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input.trim();
        if input.to_lowercase() == "exit" || input.to_lowercase() == "quit" {
            println!("Exiting");
            return;
        }

        if let Err(e) = compile_and_print(input.to_string()) {
            eprintln!("{}", e);
        }
    }
}

fn compile_and_print(source: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = Lexer::new();
    let mut parser = Parser::new();
    let ctx: Context = Context::create();
    let main_module: Module = ctx.create_module("main");
    let mut generator = Generator::new(&ctx, main_module);

    let tokens = lexer.lex(source)?;
    let ast = parser.parse(tokens)?;
    generator.generate(ast, "program".to_string())?;

    Ok(())
}

fn compile_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(filename)?;
    compile_and_print(contents)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--repl".to_string()) {
        run_repl();
        return;
    }

    if args.len() < 2 {
        eprintln!("Usage: {} <filename> [--repl]", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    if let Err(e) = compile_file(filename) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
