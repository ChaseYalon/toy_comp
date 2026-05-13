//This file contains CTLA integration tests - human written not fuzz
use chrono::Local;
use inkwell::context::Context;
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
    if !output.status.success() {
        panic!(
            "Program crashed with exit code {:?}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let s = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");
    return s;
}
macro_rules! compile_code_aot {
    ($o:ident, $i:expr, $test_name:expr) => {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_name = format!("output_{}", $test_name);
        let output_path = project_root.join("temp").join(&output_name);
        let source_path = project_root
            .join("temp")
            .join(format!("{}.toy", output_name));

        let _ = std::fs::remove_file(&output_path);
        std::fs::write(&source_path, $i).unwrap();
        thread::sleep(Duration::from_millis(100));
        let ctx = Context::create();
        let mut d =
            crate::driver::Driver::new_with_name(source_path, format!("temp/{}", output_name));
        d.start(&ctx).unwrap();

        thread::sleep(Duration::from_millis(200));

        let output_str = output_path.to_string_lossy().to_string();
        let $o = capture_program_output(output_str);
    };
}
#[test]
fn test_ctla_str() {
    compile_code_aot!(output, r#"let x = "hi"; println(x);"#, "ctla_str");
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_str_multi_block() {
    compile_code_aot!(
        output,
        r#"let x = "hi"; if x == "hello" {println("goodbye")} else {println("bye")}"#,
        "ctla_str_multi_branch"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_multi_return() {
    compile_code_aot!(
        output,
        r#"let x = "hello"; if x == "hi" {println(x); println(5)} else {println(0)}"#,
        "ctla_str_multi_return"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_arrays() {
    compile_code_aot!(output, "let arr = [1, 2, 3]; println(arr);", "ctla_arr");
    assert!(!output.contains("FAIL_TEST"))
}

#[test]
fn test_ctla_multi_func() {
    compile_code_aot!(
        output,
        r#"fn custom_concat(a: str, b: str): str {return a + b;} let x = custom_concat("hello", "world");"#,
        "ctla_func"
    );
    assert!(!output.contains("FAIL_TEST"))
}

#[test]
fn test_ctla_string_arrays() {
    compile_code_aot!(
        output,
        r#"let arr: str[][] = [["hi", "bye"], ["hello", "world"]]; arr[1][0] = "hallo"; println(arr);"#,
        "ctla_str_arr"
    );
    assert!(!output.contains("FAIL_TEST"))
}

#[test]
fn test_ctla_multi_alloc_return() {
    compile_code_aot!(
        output,
        r#"fn isEven(n: int): str {if n % 2 == 0 {return "it is";} return "it is not";} println(isEven(5));"#,
        "ctla_multi_alloc_return"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_uaf_loop_bug() {
    compile_code_aot!(
        output,
        "struct Point{
            x: float,
            y: float,
        }

        for Point {
            fn move(dx: float, dy: float) {
                this.x += dx;
                this.y += dy;
            }
        }

        let points = [
            Point{x: 0.0, y: 0.0},
            Point{x: 1.0, y: 1.0},
            Point{x: -1.0, y: -1.0}
        ];

        let i = 0;
        while i < len(points) {
            points[i].move(5.0, 0-2.0);
            i += 1;
        }
        println(points[0].x);",
        "ctla_uaf_loop_bug"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_argv() {
    compile_code_aot!(
        output,
        "import std.sys; let args = sys.argv(); println(args[0]);",
        "ctla_argv"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_ret_arr() {
    compile_code_aot!(
        output,
        "fn ret_arr(): int[] {return [1,2, 3];} let a = ret_arr(); println(a);",
        "ctla_arr_ret"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_str_reassign() {
    compile_code_aot!(
        output,
        r#"
        import std.sys;
        export fn write_response(code: int, content_type: int, body: str): str{
            let content_type_str = "";
            if content_type == 1{
                content_type_str = "text/plain; charset=utf-8";
            } else if content_type == 2{
                content_type_str = "text/html; charset=utf-8";
            } else if content_type == 3{
                content_type_str = "application/json";
            } else if content_type == 4{
                content_type_str = "application/javascript";
            } else {
                sys.panic("[ERROR] Content type you requested not implemented");
            }
            return content_type_str
        }

        println(write_response(1, 1, ""));
        println(write_response(1, 2, ""));
        "#,
        "ctla_str_reassign"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_aliasing() {
    compile_code_aot!(
        output,
        r#"
        let a = "hi";
        let b = "bye";
        let arr = [a, b]
        println(a);
        println(len(arr));
        "#,
        "ctla_aliasing"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_extern_struct_func_call() {
    compile_code_aot!(
        output,
        "import std.time; let d = time.current_date(); println(d.to_str());",
        "ctla_extern_struct_func_call"
    );
    let mut month_num = Local::now().format("%m").to_string();
    if month_num.starts_with("0") {
        month_num = month_num[1..].to_string();
    }
    assert!(output.contains(&month_num), "[DEBUG] output was {output}");
}

#[test]
fn test_ctla_struct_aliasing_and_encapsulation() {
    compile_code_aot!(
        output,
        r#"struct Test {x: str}; let s = "hello world";let m = Test{x: s}; let n = m; println(n.x);"#,
        "ctla_struct_aliasing_and_encapsulation"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_multi_module_alloc() {
    compile_code_aot!(
        output,
        r#"import std.fs; fs.write_file("temp.txt", "hi");"#,
        "ctla_multi_module"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_fs_read_dir_to_str() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_rel = format!("temp/ctla_fs_read_dir_case",);
    let case_dir = project_root.join(&case_rel);

    let _ = std::fs::remove_dir_all(&case_dir);
    std::fs::create_dir_all(case_dir.join("sub_a")).unwrap();
    std::fs::create_dir_all(case_dir.join("sub_b")).unwrap();
    std::fs::write(case_dir.join("f_a.txt"), "a").unwrap();
    std::fs::write(case_dir.join("f_b.txt"), "b").unwrap();

    let program = format!(
        r#"import std.fs; let r = fs.read_dir("{}"); println(r.to_str());"#,
        case_rel
    );
    compile_code_aot!(output, program, "ctla_fs_read_dir_to_str");

    assert!(output.contains("[Files]"), "[DEBUG] output was {output}");
    assert!(output.contains("[Folders]"), "[DEBUG] output was {output}");
    assert!(output.contains("f_a.txt"), "[DEBUG] output was {output}");
    assert!(output.contains("f_b.txt"), "[DEBUG] output was {output}");
    assert!(output.contains("sub_a"), "[DEBUG] output was {output}");
    assert!(output.contains("sub_b"), "[DEBUG] output was {output}");
    assert!(!output.contains("FAIL_TEST"));

    let _ = std::fs::remove_dir_all(&case_dir);
}

#[test]
fn test_ctla_str_lambda() {
    compile_code_aot!(
        output,
        r#"let add = (a: str, b: str): str{return a + b}; let x = add("hello ", "world"); println(x);"#,
        "ctla_str_lambda"
    );
    assert!(output.contains("hello world"));
}
