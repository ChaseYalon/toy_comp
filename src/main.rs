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
mod driver;
mod errors;
mod ffi;
use inkwell::context::Context;
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

        if let Err(e) = compile_and_run(input.to_string()) {
            eprintln!("{}", e);
        }
    }
}

fn compile_and_run(source: String) -> Result<(), Box<dyn std::error::Error>> {
    let ctx: Context = Context::create();
    let mut driver = driver::Driver::new(source);
    driver.start(&ctx)?;

    let exe_path = format!("./Program{}", driver::FILE_EXTENSION_EXE);
    let output = process::Command::new(exe_path).output()?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    Ok(())
}

fn compile_and_print(source: String) -> Result<(), Box<dyn std::error::Error>> {
    let ctx: Context = Context::create();
    let mut driver = driver::Driver::new(source);
    driver.start(&ctx)?;

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
