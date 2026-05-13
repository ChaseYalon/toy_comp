#![feature(error_generic_member_access)]
#![feature(backtrace_frames)]

use crate::driver::Driver;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
mod lexer;
pub mod parser;
mod token;
#[macro_use]
mod macros;
pub mod codegen;
pub(crate) mod driver;
mod errors;
mod ffi;
mod fuzzer;
use inkwell::context::Context;
pub use crate::parser::ast::*;
pub use crate::token::TypeTok;
pub use crate::errors::Span;
pub use crate::fuzzer::TestRunner;
pub use ordered_float::OrderedFloat;
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
    let repl_path = PathBuf::from("./temp/repl.toy");
    fs::create_dir_all("temp")?;
    fs::write(&repl_path, source)?;

    let ctx: Context = Context::create();
    let mut driver = driver::Driver::new(repl_path);
    driver.start(&ctx)?;
    let exe_path = format!("./Program{}", driver::FILE_EXTENSION_EXE);

    process::Command::new(exe_path)
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .spawn()?
        .wait()?;

    Ok(())
}
fn compile_and_print(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx: Context = Context::create();
    let args: Vec<String> = env::args().collect();
    let name = if args
        .iter()
        .position(|a| a == &"--name".to_string())
        .is_some()
    {
        args[args
            .iter()
            .position(|a| a == &"--name".to_string())
            .unwrap()
            + 1]
        .clone()
    } else {
        "program".to_string()
    };
    let mut driver = Driver::new_with_name(PathBuf::from(file_path), name);
    driver.start(&ctx)?;

    Ok(())
}

fn compile_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    compile_and_print(filename)
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--fuzz".to_string()) {
        let idx = args.iter().position(|f| f == &"--fuzz".to_string()).unwrap();
        if idx + 1 > args.len() {
            panic!("[ERROR] You should use --fuzz [NUMBER_OF_PROGRAMS]");
        }
        let num: u64 = args[idx + 1].parse().unwrap();
        for i in 0..num {
            println!("Fuzz iteration {}", i);
            let mut runner = TestRunner::new();
            println!("Generating...");
            let prgm = runner.generate();
            println!("Generated.");
            let mut d = Driver::new(PathBuf::from("temp.exe"));
            let ctx = Context::create();
            if args.contains(&"--save-temps".to_string()) {
                fs::write("temp.txt", format!("{:#?}", prgm)).unwrap();
            }
            let _ = d.start_with_ast(&ctx, prgm).unwrap();
            println!("Compiled.");
        }
        println!("All fuzzers passed");
        return;
    }
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
