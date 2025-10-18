use crate::{Compiler, Lexer, Parser};
use std::process::{Command, Stdio};

fn capture_program_output(program: String) -> String {
    let output = Command::new(program)
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run program");

    String::from_utf8(output.stdout).expect("Invalid UTF-8 output")
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
macro_rules! compile_code_aot{
    ($o:ident, $i:expr) => {
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut c = Compiler::new();
        c.compile(p.parse(l.lex($i.to_string())), false, Some("output.exe"));
        let $o = capture_program_output("output.exe".to_string());
    }
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
    compile_code_aot!(output, r#"let x = "foo"; let y = "bar; let z = x + y; println(z);"#);
    assert!(output.contains("foobar"));
}