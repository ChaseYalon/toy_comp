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
fn test_ctla_struct_field_overwrite() {
    compile_code_aot!(
        output,
        r#"
        struct P { s: str }
        let p = P{s: "init"};
        let i = 0;
        while i < 5 {
            p.s = "x";
            i = i + 1;
        }
        println(p.s);
        "#,
        "ctla_struct_field_overwrite"
    );
    assert!(!output.contains("FAIL_TEST") && !output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_struct_arr_overwrite() {
    compile_code_aot!(
        output,
        r#"
        struct P { s: str }
        let arr = [P{s: "a"}, P{s: "b"}];
        let i = 0;
        while i < 5 {
            arr[0] = P{s: "c"};
            i = i + 1;
        }
        println(arr);
        "#,
        "ctla_struct_arr_overwrite"
    );
    assert!(!output.contains("FAIL_TEST") && !output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_struct_cross_arr() {
    compile_code_aot!(
        output,
        r#"
        struct P { s: str }
        let p = P{s: "shared"};
        let a = [p];
        let b = [p];
        a[0] = P{s: "new"};
        println(b);
        println(a);
        "#,
        "ctla_struct_cross_arr"
    );
    assert!(!output.contains("FAIL_TEST") && !output.contains("FAIL_TEST"));
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
//#[ignore = "This produces weird inexplicable errors, nothing to do with ctla"]
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

#[test]
fn test_ctla_bug_1() {
    compile_code_aot!(
        output,
        r#"
        struct s1 {
            f1: bool[],
            f2: float[],
            f3: str,
            f4: int[],
            f5: int[]
        }

        fn func1(): int[]{
            let v3 = s1{
                f1: [false],
                f2: [1.0],
                f3: "", 
                f4: [-1],
                f5: [1]
            };
            v3.f1 = [false];
            return [-2];
        }

        func1();
        "#,
        "ctla_bug_1"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_2() {
    compile_code_aot!(
        output,
        r#"
        import std.fuzz;
        fn func1(p1: str[][]): str[] {
            let v1: str[] = fuzz.read_rand(p1);
            fuzz.write_arr(p1, v1);
            return ["z"];
        }
        func1([["n"]]);
        "#,
        "ctla_bug_2"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_3() {
    compile_code_aot!(
        output,
        r#"
        import std.fuzz;
        fn func1(){
            let v1: str[][] = [["z"]];
            let v2: str[] = fuzz.read_rand(v1);
            fuzz.write_arr(v1, v2);
            let v3: str[] = fuzz.read_rand(v1);
            fuzz.write_arr(v1, v3);
        }
        func1();
        "#,
        "ctla_bug_3"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_4() {
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            let v1: str[][] = [["m"]];
            if false {}
            let v2: str[] = fuzz.read_rand(v1);
        "#,
        "ctla_bug_4"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_5(){
    compile_code_aot!(
        output,
        r#"
        import std.fuzz;
        fn func1(): str[] {
            return ["B"];
        }
        func1();
        "#,
        "ctla_bug_5"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_6() {
    compile_code_aot!(
        output,
        //inexplicably this is the most reduced version, remove any element and it fails
        r#"
        if true {
            let v1: bool[] = [true];
        }
        let v2: str[][] = [["i"]];
        let v3: int = 0;
        while v3 < 100 {
            v3++;
        }

        "#,
        "ctla_bug_6"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_7(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p2: bool[]) {
                if true {
                    let v2: bool[] = [false];
                    if true {
                        let v3: bool = fuzz.read_rand(p2);
                        fuzz.write_arr(p2, v3);
                        fuzz.write_arr(v2, true);
                    }
                }
            }

            func1([false]);
        "#,
        "ctla_bug_7"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_8(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            let v1: str[] = ["s"];
            let v2 = [1.0];
            fuzz.write_arr(v1, "k");
            fuzz.write_arr(
                v1,
                "" + fuzz.read_rand(v1)
            );
        "#,
        "ctla_bug_8"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_9(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: str): float {
                let v1: str[] = [("P")];
                let v2: int = 0;
                while true {
                    fuzz.write_arr(v1, p1);
                    if v2 >= 100 {
                        break;
                    } else {
                        v2 = v2 + 1;
                    }
                }
                return 425130.0813183049;
            }

            func1("z");
        "#,
        "ctla_bug_9"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_10(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: int[], p2: int[]): int {
                if true {
                    let v1: int = 0;
                    while true {
                        let v2: int = fuzz.read_rand(p1);
                        fuzz.write_arr(p2, -2 * v2);
                        if v1 >= 100 {
                            break;
                        } else {
                            v1 = v1 + 1;
                        }
                    }
                }
                return 2;
            }

            fn func2(): float {
                let v1: int[] = [3];
                fuzz.write_arr(v1, func1(v1, v1));
                return -1.0;
            }

            func1([2], [3]);
            func2();
        "#,
        "ctla_bug_10"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_11(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: int, p3: str[]): str {
                fuzz.write_arr(p3, "e");
                return "U";
            }

            fn func2(p1: str[][], p3: str[]): int {
                let floats: float[] = [-1.0];
                fuzz.write_arr(p3, func1(-1, p3) + fuzz.read_rand(p3));
                fuzz.write_arr(floats, 3.0);
                return -2;
            }

            func1(func2([["Q"]], ["R"]), ["e"]);

        "#,
        "ctla_bug_11"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_12(){
    compile_code_aot!(
        output,
        r#"
        import std.fuzz;

        fn func1(p1: str[]): void {
            let v1: str[] = ["h"];
            fuzz.write_arr(p1, fuzz.read_rand(v1));
        }

        func1(["a"]);
        "#,
        "ctla_bug_12"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_13(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: int[]): void {
                if false {} else {
                    let v1: int[] = [1, fuzz.read_rand(p1)];
                    fuzz.write_arr(p1, fuzz.read_rand(v1));
                }
            }

            func1([1]);
        "#,
        "ctla_bug_13"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_14(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: str[][]): void {
                while true {
                    let v1: str[][] = [["A"]];
                    fuzz.write_arr(p1, fuzz.read_rand(v1));
                    break;
                }
            }

            func1([["b"]]);
        "#,
        "ctla_bug_14"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_15(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: str[][]): bool {
                return false;
            }

            fn func2(p1: bool[], p2: str[], p3: float, p4: str[][]): bool[] {
                return [
                    true,
                    false && -404505.7590752661 > 2820677330489585869,
                    false,
                    func1([["kKB", "p", "rspq"], ["gr"]]),
                    false,
                    func1([["ErMX", "R"]]),
                    true,
                    func1([["B"], ["xxJmlcd"], ["A"], ["JW", "SdzP", "O"]]),
                    false,
                    func1([["qc"]])
                ];
            }

            fn func3(p1: str[]): str[][] {
                let v1: int = 0;
                while true {
                    let v2: int = 0;
                    while true {
                        let v3: int = 0;
                        while true {
                            fuzz.write_arr(p1, "V");
                            let rd: str = fuzz.read_rand(p1);
                            fuzz.write_arr(p1, rd);
                            if v3 >= 5 {
                                break;
                            } else {
                                v3 = v3 + 1;
                            }
                        }
                        if v2 >= 5 {
                            break;
                        } else {
                            v2 = v2 + 1;
                        }
                    }
                    if v1 >= 5 {
                        break;
                    } else {
                        v1 = v1 + 1;
                    }
                }
                return [["q"]];
            }

            func2([true], ["Q"], 6014.746010345407, [[("i")]]);
            func3(["Q"]);
        "#,
        "ctla_bug_15"
    );
    assert!(!output.contains("FAIL_TEST"));
}

// Minimal version of the bug_15 pattern: an array is created in user_main, passed to a function
// that swaps/evicts elements into it (the eviction free lands in the callee), and then the SAME
// array is read back in user_main — where it is owned and deep-freed after the read. Exercises the
// param-array write path together with a caller-side read + deep-free of the survivor.
#[test]
fn test_ctla_param_array_write_then_read_in_main() {
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn writer(p1: str[]): void {
                let i: int = 0;
                while i < 5 {
                    fuzz.write_arr(p1, "V");
                    i = i + 1;
                }
            }

            let arr: str[] = ["Q"];
            writer(arr);
            println(arr);
        "#,
        "ctla_param_array_write_then_read_in_main"
    );
    assert!(!output.contains("FAIL_TEST") && !output.contains("FAIL_TEST"));
}

// The evicted value is still owned elsewhere: `y` is a live local that was also placed into `arr`,
// so `y` and `arr[0]` alias. When func1 overwrites arr[0], the displaced value must NOT be freed —
// `y` is still referenced in user_main. Freeing it at the swap is a use-after-free (and a
// double-free against `y`'s own scope).
#[test]
fn test_ctla_bug_16() {
    compile_code_aot!(
        output,
        r#"
            fn func1(p1: str[]): void {
                p1[0] = "x";
            }
            let y = "y";
            let arr = [y];
            func1(arr);
            println(y);
        "#,
        "ctla_bug_16"
    );
    assert!(!output.contains("FAIL_TEST") && !output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_double_own() {
    compile_code_aot!(
        output,
        r#"
            let arr = ["a", "b"];
            let z = arr[0];
            arr[1] = z;
            println("done");
        "#,
        "ctla_double_own"
    );
    assert!(output.contains("done"));
    assert!(!output.contains("FAIL_TEST") && !output.contains("FAIL_TST"));
}

#[test]
fn test_ctla_bug_17(){
    compile_code_aot!(
        output,
        r#"
            fn func1(): str {
                return "O";
            }
            let arr = ["Q", func1()];
        "#,
        "ctla_bug_17_arr_lit_temp"
    );
    assert!(!output.contains("FAIL_TEST"))
}

#[test]
fn test_ctla_bug_18(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(): str[][] {
                fuzz.write_arr([["j" + "w"]], fuzz.read_rand([["d"]]));
                return [["i"]];
            }

            func1();
        "#,
        "ctla_bug_18"
    );
    assert!(!output.contains("FAIL_TEST"))
}