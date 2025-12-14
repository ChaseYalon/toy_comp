use std::thread;
use std::time::Duration;
use std::process::{Command, Stdio};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::codegen::{Generator};
use inkwell::{context::Context, module::{Module}};

use std::env;
use std::path::PathBuf;

fn capture_program_output(program: String) -> String {
    thread::sleep(Duration::from_millis(100));
    let output = Command::new(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // capture stderr too
        .spawn()
        .expect("Failed to spawn process")
        .wait_with_output()
        .expect("Failed to wait on child");

    let s = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");
    return s;
}

macro_rules! compile_code_aot {
    ($o:ident, $i:expr, $test_name:expr) => {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_name = format!("output_{}", $test_name);
        let output_path = project_root.join("temp").join(&output_name);

        let _ = std::fs::remove_file(&output_path);
        thread::sleep(Duration::from_millis(100));
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let ctx: Context = Context::create();
        let main_module: Module = ctx.create_module("main");
        let mut g = Generator::new(&ctx, main_module);
        g.generate(
            p.parse(l.lex($i.to_string()).unwrap()).unwrap(),
            format!("temp/{}", output_name),
        )
        .unwrap();

        thread::sleep(Duration::from_millis(200));

        let output_str = output_path.to_string_lossy().to_string();
        let $o = capture_program_output(output_str);
    };
}

#[test]
fn test_int_lit(){
    compile_code_aot!(output, "println(5)", "int_lit");
    assert!(output.contains("5"));
}