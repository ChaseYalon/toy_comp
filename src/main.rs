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
#[cfg(feature = "profile")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;


//sort of arbitrary, tune for best results
static MAX_DELTA_DEBUG_ITERS: usize = 300;
fn collect_used_structs(nodes: &[Ast], used: &mut std::collections::HashSet<String>) {
    for node in nodes {
        match node {
            Ast::StructLit(name, fields, _) => {
                used.insert(*name.clone());
                for (_, (expr, _)) in fields.iter() {
                    collect_used_structs(&[expr.clone()], used);
                }
            }
            Ast::FuncDec(_, _, _, body, _) => collect_used_structs(body, used),
            Ast::IfStmt(cond, body, else_body, _) => {
                collect_used_structs(&[*cond.clone()], used);
                collect_used_structs(body, used);
                if let Some(eb) = else_body {
                    collect_used_structs(eb, used);
                }
            }
            Ast::WhileStmt(cond, body, _) => {
                collect_used_structs(&[*cond.clone()], used);
                collect_used_structs(body, used);
            }
            Ast::VarDec(_, _, expr, _) => collect_used_structs(&[*expr.clone()], used),
            Ast::Return(expr, _) => collect_used_structs(&[*expr.clone()], used),
            Ast::InfixExpr(l, r, _, _) => {
                collect_used_structs(&[*l.clone()], used);
                collect_used_structs(&[*r.clone()], used);
            }
            Ast::FuncCall(_, args, _) => collect_used_structs(args, used),
            Ast::EmptyExpr(e, _) | Ast::Not(e, _) => collect_used_structs(&[*e.clone()], used),
            Ast::Assignment(_, rhs, _) => collect_used_structs(&[*rhs.clone()], used),
            _ => {}
        }
    }
}
fn collect_used_vars(nodes: &[Ast], used: &mut std::collections::HashSet<String>) {
    for node in nodes {
        match node {
            Ast::VarRef(name, _) => { used.insert(*name.clone()); }
            Ast::FuncDec(_, _, _, body, _) => collect_used_vars(body, used),
            Ast::IfStmt(cond, body, else_body, _) => {
                collect_used_vars(&[*cond.clone()], used);
                collect_used_vars(body, used);
                if let Some(eb) = else_body { collect_used_vars(eb, used); }
            }
            Ast::WhileStmt(cond, body, _) => {
                collect_used_vars(&[*cond.clone()], used);
                collect_used_vars(body, used);
            }
            Ast::VarDec(_, _, expr, _) => collect_used_vars(&[*expr.clone()], used),
            Ast::Return(expr, _) => collect_used_vars(&[*expr.clone()], used),
            Ast::InfixExpr(l, r, _, _) => {
                collect_used_vars(&[*l.clone()], used);
                collect_used_vars(&[*r.clone()], used);
            }
            Ast::FuncCall(_, args, _) => collect_used_vars(args, used),
            Ast::EmptyExpr(e, _) | Ast::Not(e, _) => collect_used_vars(&[*e.clone()], used),
            Ast::Assignment(lhs, rhs, _) => {
                collect_used_vars(&[*lhs.clone()], used);
                collect_used_vars(&[*rhs.clone()], used);
            }
            Ast::MemberAccess(expr, _, _) => collect_used_vars(&[*expr.clone()], used),
            Ast::IndexAccess(expr, idx, _) => {
                collect_used_vars(&[*expr.clone()], used);
                collect_used_vars(&[*idx.clone()], used);
            }
            Ast::StructLit(_, fields, _) => {
                for (_, (expr, _)) in fields.iter() {
                    collect_used_vars(&[expr.clone()], used);
                }
            }
            Ast::ArrLit(_, elems, _) => collect_used_vars(elems, used),
            _ => {}
        }
    }
}
fn strip_unused_vars(nodes: &mut Vec<Ast>, used: &std::collections::HashSet<String>) {
    nodes.retain(|node| {
        if let Ast::VarDec(name, _, _, _) = node {
            used.contains(&**name)
        } else {
            true
        }
    });
    for node in nodes.iter_mut() {
        match node {
            Ast::FuncDec(_, _, _, body, _) => strip_unused_vars(body, used),
            Ast::IfStmt(_, body, else_body, _) => {
                strip_unused_vars(body, used);
                if let Some(eb) = else_body { strip_unused_vars(eb, used); }
            }
            Ast::WhileStmt(_, body, _) => strip_unused_vars(body, used),
            _ => {}
        }
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
    #[cfg(feature = "profile")]
    let _profiler = dhat::Profiler::new_heap();

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
        let mut base = String::new();
        for i in 0..num {
            let ms_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let mut runner = TestRunner::new_with_seed(ms_since_epoch as u64 + i);
            seed = (ms_since_epoch as u64 + i) as i64;
            let prgm = runner.generate();
            
            let prgm_clone = prgm.clone();
            base = format!("temp/fuzz{i}");
            name = format!("{base}{}", FILE_EXTENSION_EXE);
            let crashed = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
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
        }

        if let Some(prgm) = crash {
            let mut runner = TestRunner::new();
            let mut current = prgm;
            let mut count = 0;
            let mut consecutive_failures = 0;
            // compile the crashing version so fuzz0.exe on disk matches `current`
            let _ = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
                let ctx = Context::create();
                d.start_with_ast(&ctx, current.clone())
            }));
            loop {
                let reduced = runner.reduce(current.clone());

                if reduced == current || count > MAX_DELTA_DEBUG_ITERS || consecutive_failures > 20 {
                    break;
                }

                let delta_base = format!("{base}_delta");
                let delta_name = format!("{delta_base}{}", FILE_EXTENSION_EXE);
                let compile_result = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut d = Driver::new_with_name(PathBuf::from(delta_base.clone()), delta_base.clone());
                    let ctx = Context::create();
                    d.start_with_ast(&ctx, reduced.clone())
                }));

                let still_crashes = match compile_result {
                    Err(_) | Ok(Err(_)) => { consecutive_failures += 1; count += 1; continue; }
                    Ok(Ok(_)) => {
                        let child = Command::new(delta_name.clone()).spawn().expect("failed to start child");
                        !run_for_30_seconds(child)
                    }
                };

                if still_crashes {
                    current = reduced;
                    consecutive_failures = 0;
                    // recompile to the real binary path so fuzz0.exe matches `current`
                    let _ = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
                        let ctx = Context::create();
                        d.start_with_ast(&ctx, current.clone())
                    }));
                } else {
                    consecutive_failures += 1;
                }
                count += 1;
                print!("Delta debugging at iteration {count}\r");
                io::stdout().flush().unwrap();
            }

            // Try removing unused struct interfaces and var decs, but only keep if crash still reproduces
            let mut cleaned = current.clone();

            let mut used_interfaces = std::collections::HashSet::new();
            collect_used_structs(&cleaned, &mut used_interfaces);
            cleaned.retain(|node| {
                if let Ast::StructInterface(name, _, _) = node {
                    used_interfaces.contains(&**name)
                } else {
                    true
                }
            });

            let mut used_vars = std::collections::HashSet::new();
            collect_used_vars(&cleaned, &mut used_vars);
            strip_unused_vars(&mut cleaned, &used_vars);

            // Verify the cleaned version still crashes
            let delta_base = format!("{base}_delta");
            let delta_name = format!("{delta_base}{}", FILE_EXTENSION_EXE);
            let clean_ok = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d = Driver::new_with_name(PathBuf::from(delta_base.clone()), delta_base.clone());
                let ctx = Context::create();
                d.start_with_ast(&ctx, cleaned.clone())
            }));
            if let Ok(Ok(_)) = clean_ok {
                let child = Command::new(delta_name).spawn().expect("failed to start child");
                if !run_for_30_seconds(child) {
                    current = cleaned;
                    // recompile to real path
                    let _ = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
                        let ctx = Context::create();
                        d.start_with_ast(&ctx, current.clone())
                    }));
                }
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
