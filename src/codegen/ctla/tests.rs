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
        r#"import std.fs; fs.write_file("temp/temp.txt", "hi");"#,
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

#[test]
fn test_ctla_bug_19(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            let v1: str[] = ["E", fuzz.read_rand(["a", "b"])];  
        "#,
        "ctla_bug_19"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_20(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fuzz.write_arr([[false]], fuzz.read_rand([[true], [false]]));
        "#,
        "ctla_bug_20"
    );
    assert!(!output.contains("FAIL_TEST"))
}

#[test]
fn test_ctla_bug_21(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: bool[][]){
                fuzz.write_arr(p1, fuzz.read_rand(p1));
            }

            func1([[false], [false]]);
        "#,
        "ctla_bug_21"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_22(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: str) {
                let v1: str[] = ["o", fuzz.read_rand(["r"]), p1];
            }

            func1("s");
        "#,
        "ctla_bug_22"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_23(){
    compile_code_aot!(
        output,
        r#"
            struct s1 {
                f1: float[]
            }

            let v1: float[] = [1.0];
            if false {
                let v2: s1 = s1 {
                    f1: v1
                };
            }
        "#,
        "ctla_bug_23"
    );
    assert!(!output.contains("FAIL_TEST"));

    //same root cause: the inner-block struct must borrow v1, not own it, or this is a UAF
    compile_code_aot!(
        output2,
        r#"
            struct s1 {
                f1: float[]
            }

            let v1: float[] = [1.0];
            if true {
                let v2: s1 = s1 {
                    f1: v1
                };
            }
            println(v1);
        "#,
        "ctla_bug_23b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_24(){
    //the element read_rand picks moves into p1 (caller reclaims it); the one it does NOT pick
    //must still be reclaimed by the temp array's free
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: str[]) {
                fuzz.write_arr(p1, fuzz.read_rand(["a", "b"]));
            }

            func1(["J"]);
        "#,
        "ctla_bug_24"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the nested-array shape the fuzzer originally found
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;

            fn func1(p1: str[][]) {
                fuzz.write_arr(p1, fuzz.read_rand([["a"], ["b"]]));
            }

            func1([["J"]]);
        "#,
        "ctla_bug_24b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_25(){
    //an array of lambdas: the elements are function-pointer globals, never heap allocations, so
    //the array must not deep-free them (doing so corrupts the heap). capture_program_output
    //panics if the program crashes, so a clean run is the assertion.
    compile_code_aot!(
        output,
        r#"
            let v1 = [(v2: str): bool[] { return [true]; }];
        "#,
        "ctla_bug_25"
    );
    assert!(!output.contains("FAIL_TEST"));

    //a scalar lambda local must likewise never be freed as if it were a heap value
    compile_code_aot!(
        output2,
        r#"
            let v1 = (v2: int): int { return v2 + 1; };
        "#,
        "ctla_bug_25b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_26(){
    //an element read out of param p2 is owned-written into param p1: it would end up owned by both
    //caller arrays and double-freed. The source param must relinquish ownership (runtime disown).
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;

            fn func1(p1: str[], p2: str[]) {
                fuzz.write_arr(p1, fuzz.read_rand(p2));
            }

            func1(["a"], ["c"]);
        "#,
        "ctla_bug_26"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the nested-array shape the fuzzer originally found
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;

            fn func1(p1: str[][], p2: str[][]): int {
                fuzz.write_arr(p1, fuzz.read_rand(p2));
                return 1;
            }

            func1([["a"], ["b"]], [["c"]]);
        "#,
        "ctla_bug_26b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_27(){
    //an element of a heap array is read out and encapsulated into a struct field. The struct field
    //owns the element, but the outer array ALSO still owns it via its slot, so both deep-frees
    //reclaim the same inner array — a double-free. The struct field must borrow (not own) an
    //element that another live encapsulator already owns.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: float[] }
            let v1: float[][] = [[1.0]];
            let v2: s1 = s1 { f1: v1[0] };
        "#,
        "ctla_bug_27"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the read_rand shape the fuzzer originally found: the encapsulated element comes back through a
    //function return rather than a direct index
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            struct s1 { f1: float[] }
            let v1: float[][] = [[1.0]];
            let v2: s1 = s1 { f1: fuzz.read_rand(v1) };
        "#,
        "ctla_bug_27b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_28(){
    //a borrowed str param is the LEAF of a NESTED array literal `[[p1]]`. The inner array literal
    //`[p1]` owns and deep-frees its leaf at the outer array's death, but the leaf is the
    //caller-owned param — freeing it double-frees against the caller. The single-dim case `[p1]`
    //is already borrowed correctly; the gap is the inner literal of a nested one.
    compile_code_aot!(
        output,
        r#"
            fn func1(p1: str): void {
                let v2: str[][] = [[p1]];
            }
            func1("a");
        "#,
        "ctla_bug_28"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the while-loop shape the fuzzer originally found: the nested literal is rebuilt each iteration
    compile_code_aot!(
        output2,
        r#"
            fn func1(p1: str): float[] {
                let v1: int = 0;
                while true {
                    let v2: str[][] = [[p1]];
                    if v1 >= 5 { break; } else { v1 = v1 + 1; }
                }
                return [-1.0];
            }
            func1("a");
        "#,
        "ctla_bug_28b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_30(){
    //a struct literal built inside a loop emitted its scratch `alloca` in the loop body instead of
    //the function entry block. Allocas are only reclaimed at function return, so a long-running loop
    //accumulated one stack slot per iteration and overflowed the stack (segfault). The alloca must
    //be hoisted to the entry block and reused. A large bounded loop overflows the old codegen but
    //runs in O(1) stack — and terminates — once hoisted.
    compile_code_aot!(
        output,
        r#"
            struct S { a: str }
            let i: int = 0;
            while i < 1000000 {
                let s: S = S{a: "q"};
                i = i + 1;
            }
            println("done");
        "#,
        "ctla_bug_30"
    );
    assert!(output.contains("done"));
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_31(){
    //dead code drove the shallow-vs-deep free choice. Inside a constant-false block, `a` both
    //self-writes a read-out element AND receives the param `p`. The param store made
    //`array_elements_escape` treat `a`'s elements as caller-shared, so `a` was SHALLOW-freed —
    //stranding (leaking) its own owned element "u". But that block never executes, so `a` only ever
    //holds "u" and must be deep-freed. The escape analysis must ignore unreachable (constant-false)
    //blocks. Either write alone was already handled; only the combination tripped it.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: str): int {
                let v1: str[] = ["u"];
                if false {
                    fuzz.write_arr(v1, fuzz.read_rand(v1));
                    fuzz.write_arr(v1, p1);
                }
                return 0;
            }
            func1("H");
        "#,
        "ctla_bug_31"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_32(){
    //overwriting a struct's ARRAY field (`c.f1 = [..]`) freed the evicted old array with a scalar
    //`toy_free`, which drops the array box but strands its heap string elements — a leak. TirType
    //cannot distinguish str from array (both Ptr), so the eviction free must recover array-ness from
    //how the field is populated and deep-free it. int[] fields were unaffected (no heap elements).
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: str[] }
            let v1: s1 = s1{f1: ["S"]};
            v1.f1 = ["a", "b"];
        "#,
        "ctla_bug_32"
    );
    assert!(!output.contains("FAIL_TEST"));

    //nested-array field variant
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: str[][] }
            let v1: s1 = s1{f1: [["S"]]};
            v1.f1 = [["a"], ["b"]];
        "#,
        "ctla_bug_32b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_33(){
    //a caller-owned parameter written as an array element via a WRAPPER (fuzz.write_arr) was stored
    //owned, so the destination array's deep-free reclaimed the param AND the caller freed it — a
    //double-free. mark_wrapper_writes_borrowed only borrowed read-out elements; a param element must
    //also route to the borrowed wrapper clone (the wrapper analog of bug_28's direct-write case).
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: str, p2: str[]): void {
                fuzz.write_arr(p2, p1);
            }
            func1("x", ["y"]);
        "#,
        "ctla_bug_33"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the nested shape the fuzzer found: a bool[] param stored into a bool[][] param
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(p1: bool[], p2: bool[][]): int[][] {
                fuzz.write_arr(p2, p1);
                return [[-3]];
            }
            func1([true], [[false]]);
        "#,
        "ctla_bug_33b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_34(){
    //a struct field reassignment `s.f1 = v1` in a constant-false (dead) block made CTLA treat v1 as
    //encapsulated by the struct and suppress its free — but the block never executes, so v1 is never
    //actually stored and leaks. Same class as bug_31 (dead code driving free decisions), but via the
    //struct-encapsulation predicate; it too must ignore unreachable blocks. A LIVE reassignment must
    //still be recognized as encapsulation.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: int[] }
            let v1: int[] = [1];
            let v2: s1 = s1{f1: [2]};
            if false {
                v2.f1 = v1;
            }
        "#,
        "ctla_bug_34"
    );
    assert!(!output.contains("FAIL_TEST"));

    //control: the same reassignment when reachable must remain memory-safe (no leak, no double-free)
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: int[] }
            let v1: int[] = [1];
            let v2: s1 = s1{f1: [2]};
            if true {
                v2.f1 = v1;
            }
        "#,
        "ctla_bug_34b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_35(){
    //a struct element of an ARRAY-of-structs whose field is an array-element read
    //(`fuzz.read_rand(temp)`) double-freed that read-out element: the source temp deep-freed it AND
    //the struct's owned-field free reclaimed it. bug_27 borrows such a field for a lone struct, but
    //storing the struct into the array made `borrowed_struct_literal_fields` treat it as escaping
    //and own everything. A read-out field is owned by its source array regardless of the struct's
    //escape, so it must always be borrowed.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            struct s1 { f1: str[] }
            let v1: s1[] = [s1{f1: fuzz.read_rand([["a"], ["b"]])}];
        "#,
        "ctla_bug_35"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_36(){
    //a local `v1` (defined OUTSIDE the loop) written into a multi-slot parameter array `p1` via a
    //wrapper INSIDE a loop lands in several slots over the iterations (write_arr uses a random
    //index), so it ends up owned by more than one slot — the caller's deep-free reclaims it twice
    //(double-free). A value can own at most one slot, so a loop-carried write of an outer value must
    //be borrowed. Single writes were already handled; the loop is the trigger. (Fuzzer also needed a
    //second, unrelated param write and a >=2-element destination to surface it.)
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: bool[][], p2: str[]): void {
                fuzz.write_arr(p2, "x");
                let v1: bool[] = [true, false];
                let i: int = 0;
                while true {
                    fuzz.write_arr(p1, v1);
                    if (i >= 5) { break; } else { i = (i + 1); }
                }
                let j: int = 0;
                while (fuzz.read_rand(v1)) {
                    if (j >= 5) { break; } else { j = (j + 1); }
                }
            }
            func1([[false], [true]], ["y"]);
        "#,
        "ctla_bug_36"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_37(){
    //`if !true {..}` is dead, but the reachability filter (bugs 31/34) only folded a DIRECT literal
    //via resolve_iconst — `!true` is a `Not` node, so the block was treated as reachable and its
    //struct-array encapsulation of `v1` suppressed `v1`'s free → leak. Folding `Not(const)` makes
    //the branch recognised as dead. (`if false` was already handled; struct-array — not lone struct
    //— surfaced it.)
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: bool[] }
            let v1: bool[] = [false];
            if !true {
                let v2: s1[] = [s1{f1: v1}];
            }
        "#,
        "ctla_bug_37"
    );
    assert!(!output.contains("FAIL_TEST"));

    //control: the same encapsulation when reachable must stay memory-safe
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: bool[] }
            let v1: bool[] = [false];
            if !false {
                let v2: s1[] = [s1{f1: v1}];
                println(v2);
            }
        "#,
        "ctla_bug_37b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_38(){
    //overwriting a struct field that holds a BORROWED param double-freed it: the eviction reclaimed
    //the old value (the param) while the caller also freed it. Fixed by per-field struct ownership
    //tracked at runtime (mirrors the array `owned[]` model): the struct records that the field
    //borrows the param, so the eviction skips it, and the survivor is reclaimed only when owned.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: str[] }
            fn func1(p1: str[]): void {
                let v1: s1 = s1{f1: p1};
                v1.f1 = ["a", "b"];
            }
            func1(["V"]);
        "#,
        "ctla_bug_38"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the case no static analysis can decide: in a loop the same field slot is evicted holding a
    //borrowed param on some iterations and an owned array on others — only the runtime bit resolves it
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: str[] }
            fn func1(p1: str[]): void {
                let v1: s1 = s1{f1: p1};
                let i: int = 0;
                while i < 5 {
                    v1.f1 = p1;
                    v1.f1 = ["a", "b"];
                    i = i + 1;
                }
            }
            func1(["V"]);
        "#,
        "ctla_bug_38b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_39(){
    //a local array `v1` is encapsulated into a temp array (`fuzz.write_arr([[..]], v1)`) only inside
    //a dead (constant-false) branch, so `allocation_written_into_array` suppressed `v1`'s free even
    //though that write never runs → leak. Same dead-code class as bugs 31/34/37; this predicate also
    //needed the reachability filter (and to check the encapsulating array is on a live path).
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(): int {
                let v1: str[] = ["A"];
                fuzz.write_arr(v1, "Z");
                if false {
                    fuzz.write_arr([["t"]], v1);
                }
                return 0;
            }
            func1();
        "#,
        "ctla_bug_39"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_40(){
    //a caller-owned param stored in a local str[] literal (`[.., p1]`) that is then passed to
    //`fuzz.write_arr` was double-freed: `escaping_values` treated the local array as escaping merely
    //because it's passed to the wrapper, so the param write was never borrowed — and write_arr's
    //random-index overwrite could evict+free the caller's param. The wrapper escapes neither arg, so
    //escape analysis must consult the callee's escape summary, not blanket-escape all call args.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: str): bool {
                let v1: str[] = ["x", "y", p1];
                fuzz.write_arr(v1, "z");
                return true;
            }
            func1("hello");
        "#,
        "ctla_bug_40"
    );
    assert!(!output.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_41(){
    //returning a read-out element of a local array (`return fuzz.read_rand(v1)`) double-freed it:
    //the array was deep-freed at function end, reclaiming the very element being returned, so the
    //caller received a freed pointer. When a read-out element escapes via return, the source array
    //must shallow-free so the returned element survives for the caller.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(): str {
                let v1: str[] = ["y"];
                return fuzz.read_rand(v1);
            }
            func1();
        "#,
        "ctla_bug_41"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the shape the fuzzer found: the returned element flows into a caller-side array literal
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(): str {
                let v1: str[] = ["y"];
                return fuzz.read_rand(v1);
            }
            fn func2(): bool {
                let v2: str[] = ["x", func1()];
                return true;
            }
            func2();
        "#,
        "ctla_bug_41b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_42(){
    //a branch guarded by a CONSTANT COMPARISON (`1.0 >= -3`, always true) is dead, but the
    //reachability fold (bugs 31/34/37) only handled literals and `Not` — not comparisons — so the
    //dead `else` was treated as reachable and its struct encapsulation of `v1` suppressed `v1`'s
    //free → leak. resolve_const_bool now folds And/Or and comparisons (and resolve_const_num folds
    //`ItoF`, so a mixed int/float comparison folds too).
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: int[] }
            struct s2 { g1: s1 }
            let v1: int[] = [1];
            if (1.0 >= -3) {
                let v3: int = 0;
            } else {
                let v2: s2 = s2{g1: s1{f1: v1}};
            }
        "#,
        "ctla_bug_42"
    );
    assert!(!output.contains("FAIL_TEST"));

    //control: when the constant comparison makes the branch LIVE, the encapsulation must stay safe
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: int[] }
            struct s2 { g1: s1 }
            let v1: int[] = [1];
            if (1.0 >= 3.0) {
                let v3: int = 0;
            } else {
                let v2: s2 = s2{g1: s1{f1: v1}};
                println(v2.g1.f1);
            }
        "#,
        "ctla_bug_42b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_43(){
    //reading an element out of one param array and writing it into ANOTHER param array inside a
    //LOOP double-freed it: the transfer-disown model owned-transferred the element to the dest, but
    //the loop re-reads the same source slot every iteration and transfers it into several dest slots,
    //so the dest owned it multiple times → double-free at the caller's deep-free. A read-out element
    //stored into a different array inside a loop must be borrowed (the source keeps ownership).
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: bool[][], p2: bool[][]): int {
                let v1: int = 0;
                while v1 < 5 {
                    fuzz.write_arr(p1, fuzz.read_rand(p2));
                    v1 = (v1 + 1);
                }
                return 0;
            }
            func1([[false], [true]], [[false]]);
        "#,
        "ctla_bug_43"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the fuzzer's shape: a preceding self-writeback of the source array, in the same loop
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(p1: bool[][], p2: bool[][]): int {
                let v1: int = 0;
                while v1 < 5 {
                    fuzz.write_arr(p2, fuzz.read_rand(p2));
                    fuzz.write_arr(p1, fuzz.read_rand(p2));
                    v1 = (v1 + 1);
                }
                return 0;
            }
            func1([[false], [true]], [[false]]);
        "#,
        "ctla_bug_43b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_44(){
    //the same local array written into a multi-slot array by MORE THAN ONE wrapper-write site was
    //owned by several slots at once, so the outer array's deep-free reclaimed it repeatedly →
    //double-free (probabilistic on which slots the random writes hit; deterministic in a loop). An
    //element stored by >=2 wrapper writes must be borrowed — only one slot may own it.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(): int {
                let v1: float[][] = [[1.0], [3.0]];
                let v2: float[] = [2.0];
                fuzz.write_arr(v1, v2);
                fuzz.write_arr(v1, v2);
                return 0;
            }
            func1();
        "#,
        "ctla_bug_44"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the fuzzer's shape: repeated writes across loop iterations
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(): int {
                let v1: float[][] = [[1.0], [3.0]];
                let v3: int = 0;
                while v3 < 5 {
                    let v2: float[] = [2.0];
                    fuzz.write_arr(v1, v2);
                    fuzz.write_arr(v1, v2);
                    v3 = (v3 + 1);
                }
                return 0;
            }
            func1();
        "#,
        "ctla_bug_44b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_45(){
    //reading a struct FIELD (`s1.f1`, a str owned by the struct) and storing it into an array
    //double-freed it: the array claimed ownership and its deep-free reclaimed the field, then the
    //struct's field free reclaimed it again. A struct-field value stored into an array must be
    //borrowed — the struct owns and frees the field.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            struct s1 { f1: str }
            fn func1(): int {
                let v1: s1 = s1{f1: "L"};
                let v2: str[] = ["hK"];
                fuzz.write_arr(v2, v1.f1);
                return 0;
            }
            func1();
        "#,
        "ctla_bug_45"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the fuzzer's shape: field written into a throwaway array literal
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            struct s1 { f1: str, f2: float[] }
            fn func1(): float[][] {
                if true {
                    let v1: s1 = s1{f1: "L", f2: [1.0]};
                    fuzz.write_arr(["hK"], v1.f1);
                }
                return [[1.0]];
            }
            func1();
        "#,
        "ctla_bug_45b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}

#[test]
fn test_ctla_bug_29(){
    //reassigning a struct-typed field (`v1.g1 = s1{..}`) double-frees the evicted nested struct:
    //the overwrite surfaces the old value and frees it (eviction free), but the evicted struct's
    //original allocation was also left on the normal pipeline and freed again at scope end. The
    //evicted nested struct must be suppressed from its own free.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f2: int }
            struct s2 { g1: s1 }
            let v1: s2 = s2 { g1: s1 { f2: 0 } };
            v1.g1 = s1 { f2: 1 };
        "#,
        "ctla_bug_29"
    );
    assert!(!output.contains("FAIL_TEST"));

    //the shape the fuzzer originally found: nested struct with an array field, overwritten inside a
    //function that also returns an unrelated array
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: bool[], f2: bool }
            struct s2 { g1: s1 }
            fn func1(): str[][] {
                let v1: s2 = s2 { g1: s1 { f1: [true], f2: false } };
                v1.g1 = s1 { f1: [true], f2: true };
                return [["P"]];
            }
            func1();
        "#,
        "ctla_bug_29b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}