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
        .stderr(Stdio::piped()) 
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
        d.start(&ctx).unwrap();        thread::sleep(Duration::from_millis(200));        let output_str = output_path.to_string_lossy().to_string();
        let $o = capture_program_output(output_str);
    };
}
#[test]
fn test_ctla_str() {
    compile_code_aot!(output, r#"let x = "hi"; println(x);"#, "ctla_str");
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_str_multi_block() {
    compile_code_aot!(
        output,
        r#"let x = "hi"; if x == "hello" {println("goodbye")} else {println("bye")}"#,
        "ctla_str_multi_branch"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_multi_return() {
    compile_code_aot!(
        output,
        r#"let x = "hello"; if x == "hi" {println(x); println(5)} else {println(0)}"#,
        "ctla_str_multi_return"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_arrays() {
    compile_code_aot!(output, "let arr = [1, 2, 3]; println(arr);", "ctla_arr");
    assert!(!output.contains("FAIL_TEST"))
}#[test]
fn test_ctla_multi_func() {
    compile_code_aot!(
        output,
        r#"fn custom_concat(a: str, b: str): str {return a + b;} let x = custom_concat("hello", "world");"#,
        "ctla_func"
    );
    assert!(!output.contains("FAIL_TEST"))
}#[test]
fn test_ctla_string_arrays() {
    compile_code_aot!(
        output,
        r#"let arr: str[][] = [["hi", "bye"], ["hello", "world"]]; arr[1][0] = "hallo"; println(arr);"#,
        "ctla_str_arr"
    );
    assert!(!output.contains("FAIL_TEST"))
}#[test]
fn test_ctla_multi_alloc_return() {
    compile_code_aot!(
        output,
        r#"fn isEven(n: int): str {if n % 2 == 0 {return "it is";} return "it is not";} println(isEven(5));"#,
        "ctla_multi_alloc_return"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
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
}#[test]
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
}#[test]
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
}#[test]
fn test_ctla_uaf_loop_bug() {
    compile_code_aot!(
        output,
        "struct Point{
            x: float,
            y: float,
        }        for Point {
            fn move(dx: float, dy: float) {
                this.x += dx;
                this.y += dy;
            }
        }        let points = [
            Point{x: 0.0, y: 0.0},
            Point{x: 1.0, y: 1.0},
            Point{x: -1.0, y: -1.0}
        ];        let i = 0;
        while i < len(points) {
            points[i].move(5.0, 0-2.0);
            i += 1;
        }
        println(points[0].x);",
        "ctla_uaf_loop_bug"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_argv() {
    compile_code_aot!(
        output,
        "import std.sys; let args = sys.argv(); println(args[0]);",
        "ctla_argv"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_ret_arr() {
    compile_code_aot!(
        output,
        "fn ret_arr(): int[] {return [1,2, 3];} let a = ret_arr(); println(a);",
        "ctla_arr_ret"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
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
        }        println(write_response(1, 1, ""));
        println(write_response(1, 2, ""));
        "#,
        "ctla_str_reassign"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
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
}#[test]
#[ignore = "known use-after-free bug in extern struct func call CTLA handling"]
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
}#[test]
fn test_ctla_struct_aliasing_and_encapsulation() {
    compile_code_aot!(
        output,
        r#"struct Test {x: str}; let s = "hello world";let m = Test{x: s}; let n = m; println(n.x);"#,
        "ctla_struct_aliasing_and_encapsulation"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_multi_module_alloc() {
    compile_code_aot!(
        output,
        r#"import std.fs; fs.write_file("temp/temp.txt", "hi");"#,
        "ctla_multi_module"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_fs_read_dir_to_str() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_rel = format!("temp/ctla_fs_read_dir_case",);
    let case_dir = project_root.join(&case_rel);    let _ = std::fs::remove_dir_all(&case_dir);
    std::fs::create_dir_all(case_dir.join("sub_a")).unwrap();
    std::fs::create_dir_all(case_dir.join("sub_b")).unwrap();
    std::fs::write(case_dir.join("f_a.txt"), "a").unwrap();
    std::fs::write(case_dir.join("f_b.txt"), "b").unwrap();    let program = format!(
        r#"import std.fs; let r = fs.read_dir("{}"); println(r.to_str());"#,
        case_rel
    );
    compile_code_aot!(output, program, "ctla_fs_read_dir_to_str");    assert!(output.contains("[Files]"), "[DEBUG] output was {output}");
    assert!(output.contains("[Folders]"), "[DEBUG] output was {output}");
    assert!(output.contains("f_a.txt"), "[DEBUG] output was {output}");
    assert!(output.contains("f_b.txt"), "[DEBUG] output was {output}");
    assert!(output.contains("sub_a"), "[DEBUG] output was {output}");
    assert!(output.contains("sub_b"), "[DEBUG] output was {output}");
    assert!(!output.contains("FAIL_TEST"));    let _ = std::fs::remove_dir_all(&case_dir);
}#[test]
fn test_ctla_str_lambda() {
    compile_code_aot!(
        output,
        r#"let add = (a: str, b: str): str{return a + b}; let x = add("hello ", "world"); println(x);"#,
        "ctla_str_lambda"
    );
    assert!(output.contains("hello world"));
}#[test]
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
        }        fn func1(): int[]{
            let v3 = s1{
                f1: [false],
                f2: [1.0],
                f3: "", 
                f4: [-1],
                f5: [1]
            };
            v3.f1 = [false];
            return [-2];
        }        func1();
        "#,
        "ctla_bug_1"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
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
}#[test]
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
}#[test]
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
}#[test]
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
}#[test]
fn test_ctla_bug_6() {
    compile_code_aot!(
        output,
        
        r#"
        if true {
            let v1: bool[] = [true];
        }
        let v2: str[][] = [["i"]];
        let v3: int = 0;
        while v3 < 100 {
            v3++;
        }        "#,
        "ctla_bug_6"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_7(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p2: bool[]) {
                if true {
                    let v2: bool[] = [false];
                    if true {
                        let v3: bool = fuzz.read_rand(p2);
                        fuzz.write_arr(p2, v3);
                        fuzz.write_arr(v2, true);
                    }
                }
            }            func1([false]);
        "#,
        "ctla_bug_7"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
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
}#[test]
fn test_ctla_bug_9(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: str): float {
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
            }            func1("z");
        "#,
        "ctla_bug_9"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_10(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: int[], p2: int[]): int {
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
            }            fn func2(): float {
                let v1: int[] = [3];
                fuzz.write_arr(v1, func1(v1, v1));
                return -1.0;
            }            func1([2], [3]);
            func2();
        "#,
        "ctla_bug_10"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_11(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: int, p3: str[]): str {
                fuzz.write_arr(p3, "e");
                return "U";
            }            fn func2(p1: str[][], p3: str[]): int {
                let floats: float[] = [-1.0];
                fuzz.write_arr(p3, func1(-1, p3) + fuzz.read_rand(p3));
                fuzz.write_arr(floats, 3.0);
                return -2;
            }            func1(func2([["Q"]], ["R"]), ["e"]);        "#,
        "ctla_bug_11"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_12(){
    compile_code_aot!(
        output,
        r#"
        import std.fuzz;        fn func1(p1: str[]): void {
            let v1: str[] = ["h"];
            fuzz.write_arr(p1, fuzz.read_rand(v1));
        }        func1(["a"]);
        "#,
        "ctla_bug_12"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_13(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: int[]): void {
                if false {} else {
                    let v1: int[] = [1, fuzz.read_rand(p1)];
                    fuzz.write_arr(p1, fuzz.read_rand(v1));
                }
            }            func1([1]);
        "#,
        "ctla_bug_13"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_14(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: str[][]): void {
                while true {
                    let v1: str[][] = [["A"]];
                    fuzz.write_arr(p1, fuzz.read_rand(v1));
                    break;
                }
            }            func1([["b"]]);
        "#,
        "ctla_bug_14"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_15(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: str[][]): bool {
                return false;
            }            fn func2(p1: bool[], p2: str[], p3: float, p4: str[][]): bool[] {
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
            }            fn func3(p1: str[]): str[][] {
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
            }            func2([true], ["Q"], 6014.746010345407, [[("i")]]);
            func3(["Q"]);
        "#,
        "ctla_bug_15"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_param_array_write_then_read_in_main() {
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn writer(p1: str[]): void {
                let i: int = 0;
                while i < 5 {
                    fuzz.write_arr(p1, "V");
                    i = i + 1;
                }
            }            let arr: str[] = ["Q"];
            writer(arr);
            println(arr);
        "#,
        "ctla_param_array_write_then_read_in_main"
    );
    assert!(!output.contains("FAIL_TEST") && !output.contains("FAIL_TEST"));
}#[test]
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
}#[test]
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
}#[test]
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
}#[test]
fn test_ctla_bug_18(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(): str[][] {
                fuzz.write_arr([["j" + "w"]], fuzz.read_rand([["d"]]));
                return [["i"]];
            }            func1();
        "#,
        "ctla_bug_18"
    );
    assert!(!output.contains("FAIL_TEST"))
}#[test]
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
}#[test]
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
}#[test]
fn test_ctla_bug_21(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: bool[][]){
                fuzz.write_arr(p1, fuzz.read_rand(p1));
            }            func1([[false], [false]]);
        "#,
        "ctla_bug_21"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_22(){
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: str) {
                let v1: str[] = ["o", fuzz.read_rand(["r"]), p1];
            }            func1("s");
        "#,
        "ctla_bug_22"
    );
    assert!(!output.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_23(){
    compile_code_aot!(
        output,
        r#"
            struct s1 {
                f1: float[]
            }            let v1: float[] = [1.0];
            if false {
                let v2: s1 = s1 {
                    f1: v1
                };
            }
        "#,
        "ctla_bug_23"
    );
    assert!(!output.contains("FAIL_TEST"));    
    compile_code_aot!(
        output2,
        r#"
            struct s1 {
                f1: float[]
            }            let v1: float[] = [1.0];
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
}#[test]
fn test_ctla_bug_24(){
    
    
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: str[]) {
                fuzz.write_arr(p1, fuzz.read_rand(["a", "b"]));
            }            func1(["J"]);
        "#,
        "ctla_bug_24"
    );
    assert!(!output.contains("FAIL_TEST"));    
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;            fn func1(p1: str[][]) {
                fuzz.write_arr(p1, fuzz.read_rand([["a"], ["b"]]));
            }            func1([["J"]]);
        "#,
        "ctla_bug_24b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_25(){
    
    
    
    compile_code_aot!(
        output,
        r#"
            let v1 = [(v2: str): bool[] { return [true]; }];
        "#,
        "ctla_bug_25"
    );
    assert!(!output.contains("FAIL_TEST"));    
    compile_code_aot!(
        output2,
        r#"
            let v1 = (v2: int): int { return v2 + 1; };
        "#,
        "ctla_bug_25b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_26(){
    
    
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;            fn func1(p1: str[], p2: str[]) {
                fuzz.write_arr(p1, fuzz.read_rand(p2));
            }            func1(["a"], ["c"]);
        "#,
        "ctla_bug_26"
    );
    assert!(!output.contains("FAIL_TEST"));    
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;            fn func1(p1: str[][], p2: str[][]): int {
                fuzz.write_arr(p1, fuzz.read_rand(p2));
                return 1;
            }            func1([["a"], ["b"]], [["c"]]);
        "#,
        "ctla_bug_26b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_27(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_28(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_29(){
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
    assert!(!output.contains("FAIL_TEST"));    compile_code_aot!(
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
#[test]
fn test_ctla_bug_30(){
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
}#[test]
fn test_ctla_bug_31(){
    
    
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_32(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_33(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_34(){
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_35(){
    
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_36(){
    
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_37(){
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_38(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_39(){
    
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_40(){
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_41(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_42(){
    
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_43(){
    
    
    
    
    
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
}#[test]
fn test_ctla_bug_44(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_45(){
    
    
    
    
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
}#[test]
fn test_ctla_bug_46(){
    
    
    
    
    
    
    
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            struct s1 { f1: int[] }
            fn func1(): int {
                let v1: int[] = [1];
                if 5 == fuzz.read_rand([1.0]) {
                    let v2: s1[] = [s1{f1: v1}];
                }
                return 0;
            }
            func1();
        "#,
        "ctla_bug_46"
    );
    assert!(!output.contains("FAIL_TEST"));    
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            struct s1 { f1: int[], f2: bool[], f3: float, f4: int }
            fn func1(): int {
                let v1: int[] = [-3];
                if 5 == fuzz.read_rand([-2.0]) {
                    let v2: s1[] = [s1{f1: v1, f2: [true], f3: 1.0, f4: 2}];
                    let v3: int = 6;
                } else {
                    let v4: float[] = [-2.0, 7.0];
                }
                return -8;
            }
            func1();
        "#,
        "ctla_bug_46b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_47(){
    
    
    
    
    
    
    
    
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(): int[][] {
                if ((9223372036854775807 + 1) >= 0) {
                } else {
                    let v1: str[] = ["a"];
                }
                return [[1]];
            }
            func1();
        "#,
        "ctla_bug_47"
    );
    assert!(!output.contains("FAIL_TEST"));    
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(): int[][] {
                let v1: int = 0;
                while true {
                    if ((9223372036854775807 + 1) >= 0) {
                        let v2: bool[] = [true, false];
                        fuzz.write_arr(v2, fuzz.read_rand(v2));
                    } else {
                        let v3: str[] = ["a"];
                        fuzz.write_arr(v3, "b");
                    }
                    if v1 >= 5 { break; } else { v1 = (v1 + 1); }
                }
                return [[1]];
            }
            func1();
        "#,
        "ctla_bug_47b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}#[test]
fn test_ctla_bug_48(){
    
    
    
    
    
    
    
    
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: float[][]): float[] {
                let v1: int = 0;
                while true {
                    let v2: float[] = [1.0];
                    while false {
                        fuzz.write_arr(p1, v2);
                    }
                    if v1 >= 5 { break; } else { v1 = (v1 + 1); }
                }
                return [2.0];
            }
            func1([[3.0]]);
        "#,
        "ctla_bug_48"
    );
    assert!(!output.contains("FAIL_TEST"));    
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(p1: float[][]): float[] {
                let v1: int = 0;
                while true {
                    let v2: float[] = [1.0];
                    if false {
                        fuzz.write_arr(p1, v2);
                    }
                    if v1 >= 5 { break; } else { v1 = (v1 + 1); }
                }
                return [2.0];
            }
            func1([[3.0]]);
        "#,
        "ctla_bug_48b"
    );
    assert!(!output2.contains("FAIL_TEST"));    
    compile_code_aot!(
        output3,
        r#"
            import std.fuzz;
            fn func1(p1: float[][]): float[] {
                let v1: int = 0;
                while true {
                    let v2: float[] = [1.0];
                    fuzz.write_arr(p1, v2);
                    if v1 >= 5 { break; } else { v1 = (v1 + 1); }
                }
                return [2.0];
            }
            func1([[3.0]]);
        "#,
        "ctla_bug_48c"
    );
    assert!(!output3.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_49(){
    // An array literal built from a struct field read (`[v1.f1]`) must BORROW that element:
    // the string is already owned by `v1`, so the array must not free it too.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: str }
            let v1: s1 = s1{f1: "k"};
            let v2: str[] = [v1.f1];
        "#,
        "ctla_bug_49"
    );
    assert!(!output.contains("FAIL_TEST"));
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: str }
            fn func1(): float[] {
                let v1: s1 = s1{f1: "k"};
                let v2: str[] = [v1.f1];
                return [1.0];
            }
            func1();
        "#,
        "ctla_bug_49b"
    );
    assert!(!output2.contains("FAIL_TEST"));
    // The fuzzer's shape: the field read is encapsulated alongside anonymous elements,
    // which stay OWNED by the array even though `v1.f1` is only borrowed.
    compile_code_aot!(
        output3,
        r#"
            struct s1 { f1: str, f2: int[], f3: str[] }
            fn func1(): float[] {
                if true {
                    let v1: s1 = s1{f1: "k", f2: [1], f3: ["a"]};
                    let v2: str[] = ["b", v1.f1, "c"];
                }
                return [1.0];
            }
            func1();
        "#,
        "ctla_bug_49c"
    );
    assert!(!output3.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_50(){
    // The bug_49 sibling: here the array holding the field read ESCAPES (returned), so it must OWN
    // the value and the struct must disown field 0 -- borrowing would leave the escaped slot
    // dangling once the struct's field-free reclaims it.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: str }
            fn func1(): str[] {
                let v1: s1 = s1{f1: "k"};
                return [v1.f1];
            }
            let v2: str[] = func1();
        "#,
        "ctla_bug_50"
    );
    assert!(!output.contains("FAIL_TEST"));
    // The escaping array must own only the field it took: the struct's other owned fields still
    // have to be reclaimed by the struct.
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: str, f2: str }
            fn func1(): str[] {
                let v1: s1 = s1{f1: "k", f2: "j"};
                return [v1.f1];
            }
            let v2: str[] = func1();
        "#,
        "ctla_bug_50b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_51(){
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: int[], f2: str[] }
            fn func1(p1: str): str {
                let v1: str[] = [p1];
                let v2: s1[] = [s1{f1: [1], f2: v1}];
                return "O";
            }
            func1("f");
        "#,
        "ctla_bug_51"
    );
    assert!(!output.contains("FAIL_TEST"));
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: str[] }
            fn func1(p1: str): str {
                let v1: str[] = [p1];
                let v2: s1 = s1{f1: v1};
                return "O";
            }
            func1("f");
        "#,
        "ctla_bug_51b"
    );
    assert!(!output2.contains("FAIL_TEST"));
    // An escaping struct must still keep its fields owned; the borrow above must not leak into it.
    compile_code_aot!(
        output3,
        r#"
            struct s1 { f1: str[] }
            fn func1(): s1 {
                let v1: str[] = ["k"];
                return s1{f1: v1};
            }
            let v2: s1 = func1();
        "#,
        "ctla_bug_51c"
    );
    assert!(!output3.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_52(){
    // Pre-existing (fails identically before the bug_51 fix): a caller-owned param stored into an
    // array that an ESCAPING struct encapsulates. The struct leaves owning v1, whose slot owns the
    // caller's string, so the caller's value is freed twice.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: str[] }
            fn func1(p1: str): s1 {
                let v1: str[] = [p1];
                return s1{f1: v1};
            }
            let v2: s1 = func1("f");
        "#,
        "ctla_bug_52"
    );
    assert!(!output.contains("FAIL_TEST"));
    compile_code_aot!(
        output2,
        r#"
            fn func1(p1: str): str[] {
                return [p1];
            }
            let v1: str[] = func1("f");
        "#,
        "ctla_bug_52b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_53(){
    // The same array encapsulated by TWO struct literals: both structs claimed field 0 in their
    // init_owned bitmap, so both deep-freed it.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: str[] }
            let v1: str[] = ["a"];
            let v2: s1 = s1{f1: v1};
            let v3: s1 = s1{f1: v1};
        "#,
        "ctla_bug_53"
    );
    assert!(!output.contains("FAIL_TEST"));
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: str[] }
            let v1: str[] = ["a"];
            let v2: s1[] = [s1{f1: v1}, s1{f1: v1}];
        "#,
        "ctla_bug_53b"
    );
    assert!(!output2.contains("FAIL_TEST"));
    // The fuzzer's shape.
    compile_code_aot!(
        output3,
        r#"
            struct s1 { f1: float[], f2: str[] }
            fn func1(): float[][] {
                let v1: str[] = ["a"];
                let v2: s1[] = [s1{f1: [1.0], f2: v1}, s1{f1: [2.0], f2: v1}];
                return [[1.0]];
            }
            func1();
        "#,
        "ctla_bug_53c"
    );
    assert!(!output3.contains("FAIL_TEST"));
    // Control: an escaping struct sharing the value must NOT get the borrow (its fields stay owned
    // for whoever it escapes to). Same shape as bug_55 but the borrow decision is what is guarded
    // here; 55 covers the runtime deep-free of the returned struct array.
    compile_code_aot!(
        output4,
        r#"
            struct s1 { f1: str }
            fn func1(): s1 {
                let v1: str = "a";
                return s1{f1: v1};
            }
            let v2: s1 = func1();
        "#,
        "ctla_bug_53d"
    );
    assert!(!output4.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_55(){
    // Pre-existing (panics identically before the bug_52/53 fixes): the caller deep-frees a RETURNED
    // struct-element array, and toy_free_arr's subelement path has no Struct mapping in
    // to_elem_type -- runtime panic "Type Struct does not have elements". Also unresolved: v1 is
    // encapsulated by both escaping structs, so whoever frees the structs must free v1 exactly once.
    compile_code_aot!(
        output,
        r#"
            struct s1 { f1: str[] }
            fn func1(): s1[] {
                let v1: str[] = ["a"];
                return [s1{f1: v1}, s1{f1: v1}];
            }
            let v2: s1[] = func1();
        "#,
        "ctla_bug_55"
    );
    assert!(!output.contains("FAIL_TEST"));
    // Simplest trigger: any returned struct-element array, no sharing at all.
    compile_code_aot!(
        output2,
        r#"
            struct s1 { f1: str }
            fn func1(): s1[] {
                return [s1{f1: "a"}];
            }
            let v2: s1[] = func1();
        "#,
        "ctla_bug_55b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_56(){
    // Moving an element between two param arrays and back leaks it. The transfer of ownership is
    // two non-atomic halves: the owned write ACQUIRES, and a spliced toy_arr_disown(src, elem)
    // RELEASES. After the first move BOTH arrays' slots point at the element, so the write back
    // hits arr_swap_impl's self-write-back case (old == value), which preserves the destination
    // slot's prior ownership -- already cleared by the first disown -- instead of acquiring. The
    // release half still runs, so ownership is destroyed rather than transferred and the element
    // leaks.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: int[][], p2: int[][]): float {
                fuzz.write_arr(p2, fuzz.read_rand(p1));
                fuzz.write_arr(p1, fuzz.read_rand(p2));
                return 0.0;
            }
            func1([[1]], [[2]]);
        "#,
        "ctla_bug_56"
    );
    assert!(!output.contains("FAIL_TEST"));
    // Control: a CHAIN never writes into a slot that already holds the moved element, so every
    // write acquires and the transfer completes. Clean even with the bug present -- the leak is
    // specific to a cycle back into the source.
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(p1: int[][], p2: int[][], p3: int[][]): float {
                fuzz.write_arr(p2, fuzz.read_rand(p1));
                fuzz.write_arr(p3, fuzz.read_rand(p2));
                return 0.0;
            }
            func1([[1]], [[2]], [[3]]);
        "#,
        "ctla_bug_56b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_54(){
    // A read_rand result (a borrowed element of a local array) is returned, so the element escapes.
    // CTLA downgrades the source array's free from deep to SHALLOW to avoid double-freeing the
    // escaped element -- but shallow-free leaks the array's OTHER (non-escaped) elements. The array
    // has 3 strings, 1 escapes (freed by the caller), and the other 2 leak.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(): str {
                let v1: str[] = ["a", "b", "c"];
                return fuzz.read_rand(v1);
            }
            func1();
        "#,
        "ctla_bug_54"
    );
    assert!(!output.contains("FAIL_TEST"));
    // Control: a 1-element array has no survivors, so shallow-free is correct and there is no leak.
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(): str {
                let v1: str[] = ["a"];
                return fuzz.read_rand(v1);
            }
            func1();
        "#,
        "ctla_bug_54b"
    );
    assert!(!output2.contains("FAIL_TEST"));
}
#[test]
fn test_ctla_bug_57(){
    // A local array is written into a caller-owned PARAMETER array through a wrapper, so the
    // caller's array takes ownership and `func1` must not free it. `allocation_encapsulated_by_param`
    // gave up whenever the element's def block strictly DOMINATED the write, on the assumption that
    // such a write is routed to the borrowed variant -- but `mark_wrapper_writes_borrowed` never
    // borrows for a parameter destination, so the write stayed owned and the element was freed both
    // by `func1` and by the caller's deep-free.
    //
    // Any control flow at all splits the block and triggers it; the loop below is dead and is
    // deleted before codegen, yet its mere presence used to change the ownership decision. The
    // predicate has to be post-dominance (does the write actually run?), not dominance.
    compile_code_aot!(
        output,
        r#"
            import std.fuzz;
            fn func1(p1: int[][]): float {
                let v1: int[] = [1];
                while false {}
                fuzz.write_arr(p1, v1);
                return 0.0;
            }
            func1([[2]]);
        "#,
        "ctla_bug_57"
    );
    assert!(!output.contains("FAIL_TEST"));
    // Control: same shape without the block split -- def and write share a block, so dominance never
    // applied and this path was already correct.
    compile_code_aot!(
        output2,
        r#"
            import std.fuzz;
            fn func1(p1: int[][]): float {
                let v1: int[] = [1];
                fuzz.write_arr(p1, v1);
                return 0.0;
            }
            func1([[2]]);
        "#,
        "ctla_bug_57b"
    );
    assert!(!output2.contains("FAIL_TEST"));
    // Control: the write is genuinely skippable, so ownership does NOT transfer and the local must
    // keep its own free or it leaks (this is the case `test_ctla_bug_48` guards).
    compile_code_aot!(
        output3,
        r#"
            import std.fuzz;
            fn func1(p1: int[][]): float {
                let v1: int[] = [1];
                while false {
                    fuzz.write_arr(p1, v1);
                }
                return 0.0;
            }
            func1([[2]]);
        "#,
        "ctla_bug_57c"
    );
    assert!(!output3.contains("FAIL_TEST"));
}
