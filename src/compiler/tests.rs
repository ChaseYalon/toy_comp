use super::FILE_EXTENSION_EXE;
use crate::{Compiler, Lexer, Parser};
use std::env;
use std::path::PathBuf;
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
            .compile(p.parse(l.lex($i.to_string()).unwrap()).unwrap(), true, None)
            .unwrap()
            .unwrap();
    };
}
macro_rules! compile_code_aot {
    ($o:ident, $i:expr, $test_name:expr) => {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_name = format!("output_{}{}", $test_name, FILE_EXTENSION_EXE);
        let output_path = project_root.join("temp").join(&output_name);

        let _ = std::fs::remove_file(&output_path);
        thread::sleep(Duration::from_millis(100));
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut c = Compiler::new();
        c.compile(
            p.parse(l.lex($i.to_string()).unwrap()).unwrap(),
            false,
            Some(&format!("temp/{}", output_name)),
        )
        .unwrap();

        thread::sleep(Duration::from_millis(200));

        let output_str = output_path.to_string_lossy().to_string();
        let $o = capture_program_output(output_str);
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

#[test]
fn test_compiler_str_conv() {
    compile_code_aot!(
        output,
        r#"let x = "1"; let y = str(x) + "1"; println(y);"#,
        "str_concat"
    );
    assert!(output.contains("11"));
}

#[test]
fn test_compiler_bool_conv() {
    compile_code_aot!(
        output,
        r#"let x = bool("true"); let y = x && true; println(y);"#,
        "bool_conv"
    );
    assert!(output.contains("true"));
}
#[test]
fn test_compiler_int_conv() {
    compile_code_aot!(
        output,
        r#"let x = int("12"); let y = x + 2; println(y);"#,
        "int_conv"
    );
    assert!(output.contains("14"));
}

#[test]
fn test_compiler_float_infix() {
    compile_code_aot!(output, "let pi = 3 + 0.1415; println(pi);", "float_print");
    assert!(output.contains("3.1415"));
}
#[test]
fn test_compiler_float_conv() {
    compile_code_aot!(output, "let x = float(1); println(x);", "float_conv");
    assert!(output.contains("1.0"));
}

#[test]
fn test_compiler_arr_lits() {
    compile_code_aot!(output, "let x: int[] = [1, 2, 3]; println(x);", "arr_lit");
    assert!(output.contains("[1, 2, 3]"));
}

#[test]
fn test_compiler_arr_ref() {
    compile_code_aot!(
        output,
        r#"let x: str[] = ["hi", "bye", "hello", "goodbye"]; let b= x[2]; println(b)"#,
        "arr_ref"
    );
    assert!(output.contains("hello"));
}

#[test]
fn test_compiler_arr_idx_reassign() {
    compile_code_aot!(
        output,
        "let x = [1, 2, 3, 4, 5]; x[2] = 7; println(x);",
        "arr_idx_reassign"
    );
    assert!(output.contains("[1, 2, 7, 4, 5]"))
}

#[test]
fn test_compiler_arr_len() {
    compile_code_aot!(
        output,
        "let arr = [true, false, false, true]; println(len(arr));",
        "arr_len"
    );
    assert!(output.contains("4"));
}

#[test]
fn test_compiler_n_dimensional_arrays() {
    compile_code_aot!(
        output,
        "let arr: int[][] = [[1, 2], [3, 4]]; let x = arr[1][0]; println(x);",
        "nd_arr"
    );
    assert!(output.contains("3"));
}

#[test]
fn test_compiler_structs() {
    compile_code_aot!(
        output,
        "struct Foo{fee: int, baz: bool}; let x = Foo{fee: 2, baz: false}; println(x.fee);",
        "structs"
    );
    assert!(output.contains("2"));
}

#[test]
fn test_compiler_nested_structs() {
    compile_code_aot!(
        output,
        r#"struct Name{first: str, last: str}; struct Person{name: Name, age: int}; let me = Person{name: Name{first: "Chase", last: "Yalon"}, age: 15}; println(me.name.last);"#.to_string(),
        "nested_structs"
    );
    assert!(output.contains("Yalon"));
}

#[test]
fn test_compiler_struct_reassign() {
    compile_code_aot!(
        output,
        "struct Point{x: int, y: int}; let origin = Point{x: 0, y: 0}; origin.x = 8; println(origin.x);",
        "struct_reassign"
    );
    assert!(output.contains("8"));
}

#[test]
fn test_compiler_struct_func_param() {
    compile_code_aot!(
        output,
        "struct Foo{a: int}; fn bar(f: Foo): int{return f.a;} let b: int = bar(Foo{a: 1}); println(b);",
        "struct_func_param"
    );
    assert!(output.contains("1"))
}

#[test]
fn test_compiler_not(){
    compile_code_aot!(
        output,
        r#"let x = false || false; if !x{println("duh")} else {println("something has gone wrong")}"#,
        "not"
    );
    assert!(output.contains("duh"));
}