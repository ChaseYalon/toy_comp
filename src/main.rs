#![feature(error_generic_member_access)]
#![feature(backtrace_frames)]
use crate::driver::Driver;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::panic;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
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
//sort of arbitrary, tune for best results
static MAX_DELTA_DEBUG_ITERS: usize = 100;
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

        let num: u64 = args[idx + 1].parse().unwrap();
        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(num as usize);

        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicU64::new(0));
        let crash_result: Arc<Mutex<Option<Vec<Ast>>>> = Arc::new(Mutex::new(None));

        let handles: Vec<_> = (0..num_threads)
            .map(|t| {
                let stop = Arc::clone(&stop);
                let counter = Arc::clone(&counter);
                let crash_result = Arc::clone(&crash_result);

                thread::spawn(move || {
                    let r = panic::catch_unwind(std::panic::AssertUnwindSafe(|| loop {
                        if stop.load(Ordering::Acquire) {
                            return;
                        }
                        let i = counter.fetch_add(1, Ordering::Relaxed);
                        if i >= num {
                            return;
                        }

                        let seed = (t as u64)
                            .wrapping_mul(0x9e3779b97f4a7c15)
                            .wrapping_add(i);
                        let mut runner = TestRunner::new_with_seed(seed);
                        let prgm = runner.generate();
                        let thread_name = format!("fuzz_thread_{}", t);
                        let exe_path =
                            format!("{}{}", thread_name, driver::FILE_EXTENSION_EXE);

                        let prgm_clone = prgm.clone();
                        let thread_name_clone = thread_name.clone();
                        let crashed =
                            panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                let mut d = Driver::new_with_name(
                                    PathBuf::from(format!("{}.toy", thread_name_clone)),
                                    thread_name_clone,
                                );
                                let ctx = Context::create();
                                d.start_with_ast(&ctx, prgm_clone).unwrap();
                            }))
                            .is_err();

                        let _ = fs::remove_file(&exe_path);

                        if stop.load(Ordering::Acquire) {
                            return;
                        }

                        if crashed {
                            let mut lock = crash_result.lock().unwrap();
                            if lock.is_none() {
                                *lock = Some(prgm);
                            }
                            stop.store(true, Ordering::Release);
                            return;
                        }

                        println!("[thread {}] fuzz {} ok", t, i);
                    }));
                    if r.is_err() {
                        stop.store(true, Ordering::Release);
                    }
                })
            })
            .collect();

        for h in handles {
            let _ = h.join();
        }

        let crash = crash_result.lock().unwrap().take();
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
                    let mut d = Driver::new(PathBuf::from("temp.exe"));
                    let ctx = Context::create();
                    d.start_with_ast(&ctx, reduced.clone()).unwrap();
                }))
                .is_err();

                if still_crashes {
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
