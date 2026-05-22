#![feature(error_generic_member_access)]
#![feature(backtrace_frames)]
use std::env;
use std::fs;
use std::io::{self, Write};
use std::panic;
use std::path::PathBuf;
use std::process;
mod lexer;
pub mod parser;
mod token;
#[macro_use]
mod macros;
pub mod codegen;
pub mod driver;
mod errors;
mod ffi;
mod fuzzer;
pub use crate::errors::Span;
pub use crate::fuzzer::TestRunner;
pub use crate::parser::ast::*;
pub use crate::token::TypeTok;
use inkwell::context::Context;
pub use ordered_float::OrderedFloat;
pub use crate::driver::{Driver, FILE_EXTENSION_EXE};
use std::time::{UNIX_EPOCH, SystemTime};
use std::process::{Command, Child};
use std::time::{Duration, Instant};
use std::thread;
//sort of arbitrary, tune for best results
static MAX_DELTA_DEBUG_ITERS: usize = 300;
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
/// will return if the process failed or not
fn run_for_30_seconds(mut child: Child) -> bool {
    let start = Instant::now();
    let timeout = Duration::from_secs(30);

    loop {
        match child.try_wait().expect("failed to poll process") {
            Some(status) => {
                if status.success() {
                    return true;
                }
                return false;
            }
            None => {
                // Still running
                if start.elapsed() >= timeout {
                    // Still running after 30s — this is also acceptable
                    child.kill().ok(); // clean up
                    return true;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
fn compile_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    compile_and_print(filename)
}
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--from-ast-file".to_string()) {
        let idx = args.iter().position(|f| f == "--from-ast-file").unwrap();
        if args.len() < idx + 1 {
            panic!("[ERROR] You should use --from-ast-file [FILE_PATH]");
        }
        let file_path = args[idx + 1].clone();
        let prgm: Vec<Ast> =
            serde_json::from_str(fs::read_to_string(file_path).unwrap().as_str()).unwrap();
        let mut d = Driver::new(PathBuf::from("temp.exe"));
        let c = Context::create();
        let e = d.start_with_ast(&c, prgm);
        match e {
            Err(t) => eprintln!("{}", t),
            Ok(_) => {}
        };

        return;
    }
    if args.contains(&"--fuzz".to_string()) {
        let idx = args.iter().position(|f| f == "--fuzz").unwrap();

        if idx + 1 >= args.len() {
            panic!("[ERROR] You should use --fuzz [NUMBER_OF_PROGRAMS]");
        }
        let mut seed: i64 = -1;
        let num: u64 = args[idx + 1].parse().unwrap();
        let mut crash = None;
        let mut name = String::new();
        for i in 0..num {
            let ms_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let mut runner = TestRunner::new_with_seed(i * (ms_since_epoch as u64));
            seed = ((ms_since_epoch as u64) * i) as i64;
            let prgm = runner.generate();
            
            let prgm_clone = prgm.clone();
            name = format!("temp/fuzz{i}{}", FILE_EXTENSION_EXE);
            let crashed = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d = Driver::new(PathBuf::from(name.clone()));
                let ctx = Context::create();
                d.start_with_ast(&ctx, prgm_clone).unwrap();
            }))
            .is_err();
            fs::write("temp.txt", serde_json::to_string_pretty(&prgm).unwrap()).unwrap();
            if crashed {
                crash = Some(prgm);
                break;
            }
            let child = Command::new(&name.clone())
                .spawn()
                .expect("failed to start process");
            if !run_for_30_seconds(child) {
                crash = Some(prgm);
                break;
            }
            println!("fuzz {} ok", i);
        }

        if let Some(prgm) = crash {
            let mut runner = TestRunner::new();
            let mut current = prgm;
            let mut count = 0;
            loop {
                let reduced = runner.reduce(current.clone());

                if reduced == current || count > MAX_DELTA_DEBUG_ITERS {
                    break;
                }

                let still_crashes = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut d = Driver::new(PathBuf::from(name.clone()));
                    let ctx = Context::create();
                    d.start_with_ast(&ctx, reduced.clone()).unwrap();
                }))
                .is_err();

                if still_crashes || {
                    //this should short circuit
                    let child = Command::new(name.clone()).spawn().expect("failed to start child");
                    run_for_30_seconds(child)
                } {
                    current = reduced;
                }
                count += 1;
                print!("Delta debugging at iteration {count}\r");
                io::stdout().flush().unwrap();
            }

            fs::write(
                "reduced.txt",
                serde_json::to_string_pretty(&current).unwrap(),
            )
            .unwrap();
            println!("[FATAL] Fuzz failed, failing seed: {seed}");
            return;
        }

        println!("All fuzzes successful");
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
