use crate::codegen::Generator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use inkwell::{context::Context, module::Module};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
fn test_int_lit() {
    compile_code_aot!(output, "println(5);", "int_lit");
    assert!(output.contains("5"));
}

#[test]
fn test_llvm_codegen_paren_infix() {
    compile_code_aot!(output, "println((5 + 3) - 6 * 3)", "paren_infix");
    assert!(output.contains("-10"));
}

#[test]
fn test_llvm_codegen_booleans() {
    compile_code_aot!(output, "let x = true || false; println(!x);", "bools");
    assert!(output.contains("false"));
}
#[test]
fn test_llvm_codegen_nested_expr() {
    compile_code_aot!(
        output,
        "let x = 9 + 2; let y = x - 4; println(y);",
        "chained_expr"
    );
    assert!(output.contains("7"));
}

#[test]
fn test_llvm_codegen_float_stuff() {
    compile_code_aot!(output, "let x = 9.3 * 3; let y = x / 6; println(y + 2.2);", "floats");
    assert!(output.contains("6.85"));
}

#[test]
fn test_llvm_codegen_if_else() {
    compile_code_aot!(output,"if !false {print(5)} else {println(6)} print(7);", "if_else");
    assert!(output.contains("57"));
}

#[test]
fn test_llvm_codegen_if_no_else() {
    compile_code_aot!(output, "if true || false {print(3)} print(9);", "if_no_else");
    assert!(output.contains("39"));
}

#[test]
fn test_llvm_codegen_while_loop() {
    compile_code_aot!(output, "let x = 0; while x < 10 {print(x); x++}", "while_loop");
    assert!(output.contains("123456789"))
}

#[test]
fn test_llvm_codegen_funcs() {
    compile_code_aot!(
        output,
        "fn add(a: int, b: int): int { return a + b; } let x = add(5, 9); println(x);",
        "funcs"
    );
    assert!(output.contains("14"));
}

#[test]
fn test_llvm_codegen_string() {
    compile_code_aot!(
        output,
        r#"let s: str = "hello "; let t = "world"; print(s + t); println(s == t);"#,
        "string_ops"
    );
    assert!(output.contains("hello worldfalse"));
}

#[test]
fn test_llvm_arrays() {
    compile_code_aot!(
        output,
        r#"let arr: int[] = [1, 2, 3, 4, 5]; arr[2] = 9; print(arr[2]); println(len(arr));"#,
        "arrays"
    );
    assert!(output.contains("95"));
}

#[test]
fn test_llvm_codegen_structs(){
    compile_code_aot!(
        output,
        r#"
        struct Point {x: int, y: int};
        let p = Point { x: 5, y: 10 };
        print(p.x);
        print(p.y);
        p.x = 20;
        println(p.x);
        "#,
        "structs"
    );
    assert!(output.contains("51020"));
}

#[test]
fn test_llvm_codegen_nested_array() {
    compile_code_aot!(
        output,
        r#"
        let arr: int[][] = [[1,2,3],[4,5,6],[7,8,9]];
        print(arr[1][1]);
        arr[2][2] = 42;
        print(arr[2][2]);
        print(len(arr));
        println(arr);
        "#,
        "nested_arrays"
    );
    assert!(output.contains("5423[[1, 2, 3], [4, 5, 6], [7, 8, 42]]"));
}

#[test]
fn test_llvm_codegen_nested_structs() {
    compile_code_aot!(
        output,
        r#"
        struct Point {x: int, y: int};
        struct Circle {center: Point, radius: int};
        let c = Circle { center: Point { x: 3, y: 4 }, radius: 10 };
        print(c.center.x);
        print(c.center.y);
        println(c.radius);
        "#,
        "nested_structs"
    );
    assert!(output.contains("3410"));
}