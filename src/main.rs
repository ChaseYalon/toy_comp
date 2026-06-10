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
pub use crate::driver::{Driver, FILE_EXTENSION_EXE};
pub use crate::errors::Span;
pub use crate::fuzzer::TestRunner;
pub use crate::parser::ast::*;
pub use crate::token::TypeTok;
use inkwell::context::Context;
pub use ordered_float::OrderedFloat;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "profile")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

//sort of arbitrary, tune for best results
static MAX_DELTA_DEBUG_ITERS: usize = 5000;
static MAX_CONSECUTIVE_FAILURES: usize = 100;
static MAX_REDUCE_SECS: u64 = 300; // 5-minute wall-clock limit per reduction run
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
            Ast::VarRef(name, _) => {
                used.insert(*name.clone());
            }
            Ast::FuncDec(_, _, _, body, _) => collect_used_vars(body, used),
            Ast::IfStmt(cond, body, else_body, _) => {
                collect_used_vars(&[*cond.clone()], used);
                collect_used_vars(body, used);
                if let Some(eb) = else_body {
                    collect_used_vars(eb, used);
                }
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
fn strip_unused(nodes: &mut Vec<Ast>) {
    let mut used_vars = std::collections::HashSet::new();
    collect_used_vars(nodes, &mut used_vars);
    strip_unused_vars(nodes, &used_vars);
    let mut used_ifaces = std::collections::HashSet::new();
    collect_used_structs(nodes, &mut used_ifaces);
    nodes.retain(|node| {
        if let Ast::StructInterface(name, _, _) = node {
            used_ifaces.contains(&**name)
        } else {
            true
        }
    });
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
                if let Some(eb) = else_body {
                    strip_unused_vars(eb, used);
                }
            }
            Ast::WhileStmt(_, body, _) => strip_unused_vars(body, used),
            _ => {}
        }
    }
}
/// Returns `(stmt_count, expr_count)` by recursively walking the AST.
/// Statements: VarDec, IfStmt, WhileStmt, FuncDec, Return, Assignment, Break, Continue
/// Expressions: literals, VarRef, InfixExpr, Not, EmptyExpr, FuncCall, ArrLit, StructLit,
///              MemberAccess, IndexAccess
fn count_nodes(nodes: &[Ast]) -> (usize, usize) {
    nodes.iter().fold((0, 0), |(s, e), n| {
        let (ns, ne) = count_node(n);
        (s + ns, e + ne)
    })
}
fn count_node(node: &Ast) -> (usize, usize) {
    match node {
        Ast::VarDec(_, _, expr, _) => {
            let (s, e) = count_node(expr);
            (1 + s, e)
        }
        Ast::IfStmt(cond, body, else_body, _) => {
            let (cs, ce) = count_node(cond);
            let (bs, be) = count_nodes(body);
            let (es, ee) = else_body.as_ref().map(|b| count_nodes(b)).unwrap_or((0, 0));
            (1 + cs + bs + es, ce + be + ee)
        }
        Ast::WhileStmt(cond, body, _) => {
            let (cs, ce) = count_node(cond);
            let (bs, be) = count_nodes(body);
            (1 + cs + bs, ce + be)
        }
        Ast::FuncDec(_, _, _, body, _) => {
            let (bs, be) = count_nodes(body);
            (1 + bs, be)
        }
        Ast::Return(expr, _) => {
            let (s, e) = count_node(expr);
            (1 + s, e)
        }
        Ast::Assignment(lhs, rhs, _) => {
            let (ls, le) = count_node(lhs);
            let (rs, re) = count_node(rhs);
            (1 + ls + rs, le + re)
        }
        Ast::Break(_) | Ast::Continue(_) => (1, 0),
        Ast::IntLit(..) | Ast::FloatLit(..) | Ast::BoolLit(..) | Ast::StringLit(..)
        | Ast::VarRef(..) => (0, 1),
        Ast::InfixExpr(l, r, _, _) => {
            let (ls, le) = count_node(l);
            let (rs, re) = count_node(r);
            (ls + rs, 1 + le + re)
        }
        Ast::Not(e, _) | Ast::EmptyExpr(e, _) => {
            let (s, e2) = count_node(e);
            (s, 1 + e2)
        }
        Ast::FuncCall(_, args, _) => {
            let (s, e) = count_nodes(args);
            (s, 1 + e)
        }
        Ast::ArrLit(_, elems, _) => {
            let (s, e) = count_nodes(elems);
            (s, 1 + e)
        }
        Ast::StructLit(_, fields, _) => {
            let (s, e) = fields
                .values()
                .fold((0, 0), |(s, e), (expr, _)| {
                    let (ns, ne) = count_node(expr);
                    (s + ns, e + ne)
                });
            (s, 1 + e)
        }
        Ast::MemberAccess(expr, _, _) => {
            let (s, e) = count_node(expr);
            (s, 1 + e)
        }
        Ast::IndexAccess(expr, idx, _) => {
            let (s1, e1) = count_node(expr);
            let (s2, e2) = count_node(idx);
            (s1 + s2, 1 + e1 + e2)
        }
        _ => (0, 0),
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
//just random algo for randomness
fn make_seed(counter: u64, ms: u64) -> u64 {
    let mut x = ms ^ counter.wrapping_mul(0x9e3779b97f4a7c15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    return x;
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

        let base = "temp/from_ast".to_string();
        let name = format!("{base}{}", FILE_EXTENSION_EXE);

        let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
        let ctx = Context::create();
        d.start_with_ast(&ctx, prgm).unwrap();

        let child = Command::new(&name)
            .spawn()
            .expect("failed to start process");
        run_for_30_seconds(child);
        return;
    }
    if args.contains(&"--fuzz".to_string()) {
        let idx = args.iter().position(|f| f == "--fuzz").unwrap();

        if idx + 1 >= args.len() {
            panic!("[ERROR] You should use --fuzz [NUMBER_OF_PROGRAMS]");
        }
        let mut seed: i64 = -1;
        let num: u64 = args[idx + 1].parse().unwrap();
        let mut crash: Option<(Vec<Ast>, bool)> = None; // (program, crash_is_compile_panic)
        let mut name = String::new();
        let mut base = String::new();
        for i in 0..num {
            seed = if args.contains(&"--with-seed".to_string()) {
                let idx = args.iter().position(|f| f == "--with-seed").unwrap();

                if idx + 1 >= args.len() {
                    panic!("[ERROR] You should use --with_seed [POSITIVE_64_BIT_INTEGER]");
                }
                args[idx + 1].parse().unwrap()
            } else {
                let ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                make_seed(i, ms as u64) as i64
            };
            let mut runner = TestRunner::new_with_seed(seed as u64);
            let prgm = runner.generate();

            let prgm_clone = prgm.clone();
            base = format!("temp/fuzz{i}");
            name = format!("{base}{}", FILE_EXTENSION_EXE);
            let compile_result = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
                let ctx = Context::create();
                d.start_with_ast(&ctx, prgm_clone).unwrap();
            }));
            let crashed = compile_result.is_err();
            fs::write("temp.txt", serde_json::to_string_pretty(&prgm).unwrap()).unwrap();
            if crashed {
                crash = Some((prgm, true));
                break;
            }
            let child = Command::new(&name.clone())
                .spawn()
                .expect("failed to start process");
            if !run_for_30_seconds(child) {
                crash = Some((prgm, false));
                break;
            }
            println!("Fuzz {i} completed");
        }

        if let Some((prgm, crash_is_compile_panic)) = crash {
            let mut runner = TestRunner::new();
            let mut current = prgm;
            let mut count = 0;
            let mut consecutive_failures = 0;
            let reduce_start = Instant::now();
            // compile the crashing version so fuzz0.exe on disk matches `current`
            let _ = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
                let ctx = Context::create();
                d.start_with_ast(&ctx, current.clone())
            }));
            loop {
                let mut reduced = match count % 5 {
                    0 => runner.reduce(current.clone()),
                    1 => runner.reduce_body(current.clone()),
                    2 => runner.reduce_top_level_call(current.clone()),
                    3 => runner.simplify_func_body(current.clone()),
                    _ => runner.reduce_params(current.clone()),
                };
                // Strip unused vars/interfaces on the candidate BEFORE the crash
                // check — unused allocations can change CTLA behavior, so the
                // stripped form must be what gets verified.
                strip_unused(&mut reduced);

                if count > MAX_DELTA_DEBUG_ITERS
                    || consecutive_failures > MAX_CONSECUTIVE_FAILURES
                    || reduce_start.elapsed() > Duration::from_secs(MAX_REDUCE_SECS)
                {
                    break;
                }

                if reduced == current {
                    consecutive_failures += 1;
                    count += 1;
                    continue;
                }

                let delta_base = format!("{base}_delta");
                let delta_name = format!("{delta_base}{}", FILE_EXTENSION_EXE);
                let compile_result = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut d = Driver::new_with_name(
                        PathBuf::from(delta_base.clone()),
                        delta_base.clone(),
                    );
                    let ctx = Context::create();
                    d.start_with_ast(&ctx, reduced.clone())
                }));

                let still_crashes = match compile_result {
                    Err(_) | Ok(Err(_)) => crash_is_compile_panic,
                    Ok(Ok(_)) => {
                        if crash_is_compile_panic {
                            // compiled successfully — no longer triggers the compiler crash
                            false
                        } else {
                            let child = Command::new(delta_name.clone())
                                .spawn()
                                .expect("failed to start child");
                            !run_for_30_seconds(child)
                        }
                    }
                };

                if still_crashes {
                    current = reduced;
                    consecutive_failures = 0;
                    // checkpoint so a reducer crash never loses accepted progress
                    fs::write(
                        "reduced.txt",
                        serde_json::to_string_pretty(&current).unwrap(),
                    )
                    .unwrap();
                    // recompile to the real binary path so fuzz0.exe matches `current`
                    let _ = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut d =
                            Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
                        let ctx = Context::create();
                        d.start_with_ast(&ctx, current.clone())
                    }));
                } else {
                    consecutive_failures += 1;
                }
                count += 1;
                let (stmts, exprs) = count_nodes(&current);
                print!("Delta debugging at iteration {count} ({stmts} stmts, {exprs} exprs)\r");
                io::stdout().flush().unwrap();
                if stmts < 10 && exprs < 5 {
                    break;
                }
            }

            // Try removing unused struct interfaces and var decs, but only keep if crash still reproduces
            let mut cleaned = current.clone();
            strip_unused(&mut cleaned);

            // Verify the cleaned version still crashes
            let delta_base = format!("{base}_delta");
            let delta_name = format!("{delta_base}{}", FILE_EXTENSION_EXE);
            let clean_ok = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d =
                    Driver::new_with_name(PathBuf::from(delta_base.clone()), delta_base.clone());
                let ctx = Context::create();
                d.start_with_ast(&ctx, cleaned.clone())
            }));
            if let Ok(Ok(_)) = clean_ok {
                let child = Command::new(delta_name)
                    .spawn()
                    .expect("failed to start child");
                if !run_for_30_seconds(child) {
                    current = cleaned;
                    // recompile to real path
                    let _ = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut d =
                            Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
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

    if args.contains(&"--reduce-from-ast".to_string()) {
        let idx = args.iter().position(|f| f == "--reduce-from-ast").unwrap();
        if idx + 1 >= args.len() {
            panic!("[ERROR] You should use --reduce-from-ast [AST_FILE]");
        }
        let ast_file = &args[idx + 1];
        let ast_json = fs::read_to_string(ast_file).expect("failed to read AST file");
        let prgm: Vec<Ast> =
            serde_json::from_str(&ast_json).expect("failed to deserialize AST file");

        let base = "temp/reduce".to_string();
        let name = format!("{base}{}", FILE_EXTENSION_EXE);
        let max_reduce_secs: u64 = env::var("TOY_REDUCE_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(MAX_REDUCE_SECS);

        // None = any crash counts (--assume-crash skips the expensive initial
        // verification compile), Some(true) = compile panic, Some(false) = runtime crash
        let crash_is_compile_panic: Option<bool> = if args
            .contains(&"--assume-crash".to_string())
        {
            println!("Skipping initial verification (--assume-crash): any crash counts");
            None
        } else {
            // verify it actually crashes first, and record whether it's a compile panic
            let compile_result = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
                let ctx = Context::create();
                d.start_with_ast(&ctx, prgm.clone())
            }));
            let is_compile_panic = matches!(compile_result, Err(_) | Ok(Err(_)));
            let crashes = match compile_result {
                Err(_) | Ok(Err(_)) => true,
                Ok(Ok(_)) => {
                    let child = Command::new(&name)
                        .spawn()
                        .expect("failed to start process");
                    !run_for_30_seconds(child)
                }
            };
            if !crashes {
                println!("AST does not crash, nothing to reduce");
                return;
            }
            println!(
                "Crash type: {}",
                if is_compile_panic { "compiler panic" } else { "runtime crash" }
            );
            Some(is_compile_panic)
        };

        let test_crashes = |candidate: &[Ast]| -> bool {
            let delta_base = format!("{base}_delta");
            let delta_name = format!("{delta_base}{}", FILE_EXTENSION_EXE);
            let compile_result = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut d =
                    Driver::new_with_name(PathBuf::from(delta_base.clone()), delta_base.clone());
                let ctx = Context::create();
                d.start_with_ast(&ctx, candidate.to_vec())
            }));
            match compile_result {
                // a compiler panic counts as a crash in assume mode, but a clean
                // compile error just means the reduction produced an invalid program
                Err(_) => crash_is_compile_panic.unwrap_or(true),
                Ok(Err(_)) => crash_is_compile_panic.unwrap_or(false),
                Ok(Ok(_)) => {
                    if crash_is_compile_panic == Some(true) {
                        false
                    } else {
                        let child = Command::new(delta_name)
                            .spawn()
                            .expect("failed to start child");
                        !run_for_30_seconds(child)
                    }
                }
            }
        };

        // time-based by default so a rerun after a reducer crash explores a different
        // mutation path instead of deterministically replaying into the same crash
        let reduce_seed: u64 = env::var("TOY_REDUCE_SEED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| {
                let ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                make_seed(0, ms)
            });
        println!("Reducer seed: {reduce_seed}");
        let mut runner = TestRunner::new_with_seed(reduce_seed);
        let mut current = prgm;
        let mut count = 0;
        let mut consecutive_failures = 0;
        let reduce_start = Instant::now();

        // Phase 1: remove many functions per candidate so the program (and the
        // superlinear CTLA compile time) shrinks geometrically before the
        // one-mutation-per-compile loop below.
        let func_count =
            |nodes: &[Ast]| nodes.iter().filter(|n| n.node_type() == "FuncDec").count();
        let mut chunk = func_count(&current) / 2;
        while chunk >= 1 && reduce_start.elapsed() < Duration::from_secs(max_reduce_secs) {
            let mut progressed = false;
            for _ in 0..3 {
                if reduce_start.elapsed() >= Duration::from_secs(max_reduce_secs) {
                    break;
                }
                let mut candidate = current.clone();
                for _ in 0..chunk {
                    candidate = runner.reduce(candidate);
                }
                strip_unused(&mut candidate);
                if candidate == current {
                    continue;
                }
                let (stmts, exprs) = count_nodes(&candidate);
                println!(
                    "Phase 1: trying removal of {chunk} functions ({stmts} stmts, {exprs} exprs)"
                );
                if test_crashes(&candidate) {
                    current = candidate;
                    // checkpoint so a reducer crash never loses accepted progress
                    fs::write(
                        "reduced.txt",
                        serde_json::to_string_pretty(&current).unwrap(),
                    )
                    .unwrap();
                    progressed = true;
                    break;
                }
            }
            if progressed {
                chunk = (func_count(&current) / 2).min(chunk);
            } else {
                chunk /= 2;
            }
        }
        let (stmts, exprs) = count_nodes(&current);
        println!("Phase 1 done ({stmts} stmts, {exprs} exprs), starting fine-grained reduction");

        loop {
            let mut reduced = match count % 4 {
                0 => runner.reduce(current.clone()),
                1 => runner.reduce_body(current.clone()),
                2 => runner.reduce_top_level_call(current.clone()),
                _ => runner.simplify_func_body(current.clone()),
            };
            // Strip unused vars/interfaces on the candidate BEFORE the crash
            // check — unused allocations can change CTLA behavior, so the
            // stripped form must be what gets verified.
            strip_unused(&mut reduced);

            if count > MAX_DELTA_DEBUG_ITERS
                || consecutive_failures > MAX_CONSECUTIVE_FAILURES
                || reduce_start.elapsed() > Duration::from_secs(max_reduce_secs)
            {
                break;
            }

            if reduced == current {
                consecutive_failures += 1;
                count += 1;
                continue;
            }

            if test_crashes(&reduced) {
                current = reduced;
                consecutive_failures = 0;
                // checkpoint so a reducer crash never loses accepted progress
                fs::write(
                    "reduced.txt",
                    serde_json::to_string_pretty(&current).unwrap(),
                )
                .unwrap();
            } else {
                consecutive_failures += 1;
            }
            count += 1;
            let (stmts, exprs) = count_nodes(&current);
            print!("Delta debugging at iteration {count} ({stmts} stmts, {exprs} exprs)\r");
            io::stdout().flush().unwrap();
            if stmts < 10 && exprs < 5 {
                break;
            }
        }

        // cleanup pass
        let mut cleaned = current.clone();
        strip_unused(&mut cleaned);
        if cleaned != current && test_crashes(&cleaned) {
            current = cleaned;
        }

        // recompile the final result to the real path so temp/reduce.exe matches reduced.txt
        let _ = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut d = Driver::new_with_name(PathBuf::from(base.clone()), base.clone());
            let ctx = Context::create();
            d.start_with_ast(&ctx, current.clone())
        }));

        fs::write(
            "reduced.txt",
            serde_json::to_string_pretty(&current).unwrap(),
        )
        .unwrap();
        println!("Reduced AST written to reduced.txt ({count} iterations)");
        return;
    }

    if args.contains(&"--dump-ast".to_string()) {
        let idx = args.iter().position(|f| f == "--dump-ast").unwrap();
        if idx + 1 >= args.len() {
            panic!("[ERROR] You should use --dump-ast [TOY_FILE]");
        }
        let mut d = Driver::new_with_name(PathBuf::from(&args[idx + 1]), "temp/dump".to_string());
        let ast = d.parse_only().unwrap();
        println!("{}", serde_json::to_string_pretty(&ast).unwrap());
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
