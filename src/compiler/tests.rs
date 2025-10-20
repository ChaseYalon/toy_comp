use crate::{Compiler, Lexer, Parser};
use std::env;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
macro_rules! compile_code {
    ($o:ident, $i:expr) => {
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut c = Compiler::new();
        let $o = c
            .compile(p.parse(l.lex($i.to_string())), true, None)
            .unwrap();
    };
}
macro_rules! compile_code_aot {
    ($o:ident, $i:expr, $test_name:expr) => {
        let project_root_str = env!("CARGO_MANIFEST_DIR");
        let output_name = format!("output_{}.exe", $test_name);
        let output_path = format!("{}\\temp\\{}", project_root_str, output_name);

        let _ = std::fs::remove_file(&output_path);
        thread::sleep(Duration::from_millis(100)); // Wait for file deletion

        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut c = Compiler::new();
        c.compile(
            p.parse(l.lex($i.to_string())),
            false,
            Some(&format!("temp/{}", output_name)),
        );

        thread::sleep(Duration::from_millis(200)); // Wait for compilation
        let $o = capture_program_output(output_path);
    };
}
#[test]
fn test_compiler_int_lit() {
    compile_code!(code_fn, "6");
    assert_eq!(6, code_fn());
}

#[test]
fn test_compiler_int_multi_char_lit() {
    compile_code!(code_fn, "16");
    assert_eq!(16, code_fn());
}

#[test]
fn test_compiler_int_infix_1() {
    compile_code!(code_fn, "18 - 3");
    assert_eq!(15, code_fn());
}

#[test]
fn test_compiler_int_infix_2() {
    compile_code!(code_fn, "24 / 6 - 3");
    assert_eq!(1, code_fn());
}
#[test]
fn test_compiler_var_ref() {
    compile_code!(code_fn, "let x = 1; x;");
    assert_eq!(1, code_fn())
}
#[test]
fn test_compiler_var_ref_reassign() {
    compile_code!(code_fn, "let x = 9; x = 2; x;");
    assert_eq!(2, code_fn());
}

#[test]
fn test_compiler_static_types() {
    compile_code!(code_fn, "let x: int = 9; x;");
    assert_eq!(9, code_fn());
}

#[test]
fn test_compiler_bool_literal() {
    compile_code!(code_fn, "let b: bool = true; b;");
    assert_eq!(1, code_fn());
}

#[test]
fn test_compiler_bool_infix() {
    compile_code!(code_fn, "let b = 7 < 8 || false; b;");
    assert_eq!(1, code_fn());
}

#[test]
fn test_compiler_if_stmt() {
    compile_code!(code_fn, "let x:int = 8; if true {x = 4}; x;");
    assert_eq!(4, code_fn());
}

#[test]
fn test_compiler_if_else() {
    compile_code!(code_fn, "let x = 10; if x < 9 {x = 12;} else {x = 13;} x;");
    assert_eq!(13, code_fn());
}

#[test]
fn test_compiler_nested_parens() {
    compile_code!(code_fn, "let x = (5 * (3 + 4)) / 7; x;");
    assert_eq!(5, code_fn());
}
#[test]
fn test_compiler_func_dec_and_call() {
    compile_code!(
        code_fn,
        "fn add(a: int, b: int): int {return a + b;} let x = add(2, 4); x;"
    );
    assert_eq!(6, code_fn());
}

#[test]
fn test_compiler_if_in_func() {
    compile_code!(
        code_fn,
        "fn addIfEven(a: int, b: int): int {if a % 2 == 0 && b % 2 == 0 {return a + b;} else {return 0;}} let x = addIfEven(8, 2); x;"
    );
    assert_eq!(10, code_fn())
}
#[test]
fn test_compiler_string_concat() {
    compile_code_aot!(
        output,
        r#"let x = "foo"; let y = "bar"; let z = x + y; println(z);"#,
        "string_concat"
    );
    assert!(output.contains("foobar"));
}

#[test]
fn test_compiler_print_bool() {
    compile_code_aot!(output, r#"print(true);"#, "print_bool");
    assert!(output.contains("true"));
}

#[test]
fn test_compiler_strlen() {
    compile_code_aot!(output, r#"println(len("hi"));"#, "strlen");
    assert!(output.contains("2"));
}
#[test]
fn test_compiler_while() {
    compile_code!(
        code_fn,
        "let x = 0; while x < 10 {if x == 0{x++;continue;} if x == 7{break;} x++;} x;"
    );
    assert_eq!(code_fn(), 7)
}
