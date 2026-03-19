use super::AliasAndEncapsulationTracker;
use crate::codegen::ctla::CTLA;
use crate::codegen::tir::AstToIrConverter;
use crate::Driver;
use crate::parser::ast::Ast;
use crate::parser::ast_gen::AstGenerator;
use std::path::PathBuf;

fn parse_test_code(code: impl ToString) -> (Vec<Ast>, Driver) {
    let mut driver = Driver::new(PathBuf::from("temp/test.toy"));
    let mut ast_gen = AstGenerator::new();
    let ast = driver
        .compile_to_ast_from_str(code.to_string(), &mut ast_gen)
        .unwrap();
    (ast, driver)
}

macro_rules! setup_tir_builder {
    ($o: ident, $v:expr) => {
        let (ast, driver) = parse_test_code($v);
        let mut t = AstToIrConverter::new();
        for (path, exports) in &driver.table.path_to_exports {
            let module_name = path
                .replace("/", ".")
                .replace(".toy", "")
                .trim_start_matches('.')
                .to_string();
            let prefix = module_name.replace(".", "::");
            for export in exports {
                if let crate::driver::ModuleExportType::Function(_params, ret) = &export.ty {
                    let full_mangled =
                        crate::driver::Driver::mangle_name(Some(&prefix), &export.name, &[]);
                    t.builder.register_extern_func(full_mangled, ret.clone(), true);//i think true is right?
                }
            }
        }
        eprintln!("{:#?}", t.convert(ast, true, "test").unwrap());
        let mut analyzer = CTLA::new();
        analyzer.analyze(t.builder.clone()).unwrap();
        let $o: AliasAndEncapsulationTracker = analyzer.alias_tracker().clone();
    };
}

#[test]
fn test_basic_alias(){
    setup_tir_builder!(tracker, r#"let x = "foo"; fn dup(x: str): str{return x}; let y = dup(x); println(y);"#);

    assert!(tracker.aliases.contains(&(1, "user_main".to_string(), 3)));
}

#[test]
fn test_basic_encapsulator(){
    setup_tir_builder!(tracker, r#"let x = "hi"; let arr = [x]; println(arr); println(x);"#);
    assert!(tracker.has_encapsulator(1, "user_main", 8), "{:#?}", tracker.encapsulators);
}

#[test]
fn test_function_alias(){
    setup_tir_builder!(tracker, r#"let x = "bye"; fn dup(x: str): str{ return x}; let y = dup(x); println(y);"#);
    assert!(tracker.has_alias(1, "user_main", 3), "{:#?}", tracker.aliases);
}

#[test]
fn test_partial_function_alias() {
    setup_tir_builder!(tracker, r#"let name = "chase"; fn partial(s: str, i: int): str { if i % 2 == 0{return s} return "";} let y = partial(name, 2); println(y);"#);
    assert!(tracker.has_alias(1, "user_main", 5), "{:#?}", tracker.aliases)
}

#[test]
fn test_partial_function_encapsulator(){
    setup_tir_builder!(tracker, r#"let sister = "ella"; fn partial(s: str, i: int): str[] {if i % 3 == 0 {let arr = [s, "hello"]; return arr} let arr: str[] = []; return arr} let v = partial(sister, 2);"#);
    assert!(tracker.has_encapsulator(1, "user_main", 5))
}

#[test]
fn test_multi_alias(){
    setup_tir_builder!(tracker, r#"let a = "hello"; fn dup(x: str): str{return x};let b = dup(a); let c = dup(b);"#);
    assert!(tracker.has_alias(1, "user_main", 3));
    assert!(tracker.has_alias(1, "user_main", 4));
}

#[test]
fn test_branch_alias_and_encapsulator(){
    setup_tir_builder!(tracker, r#"let x = "hi"; let arr: str[] = []; let y = "";if 3 % 2 == 0{arr = [x]} else {y = x}"#);

    assert!(tracker.has_alias(1, "user_main", 24), "{:#?}", tracker.aliases);
    assert!(
        tracker.has_encapsulator(1, "user_main", 20),
        "{:#?}",
        tracker.encapsulators
    );
}

#[test]
fn test_nested_encapsulator(){
    setup_tir_builder!(tracker, r#"let arr = ["hello"]; let outer = [arr];"#);
    assert!(tracker.has_encapsulator(1, "user_main", 8));
    assert!(tracker.has_encapsulator(5, "user_main", 15), "{:#?}", tracker.encapsulators)
}

#[test]
fn test_local_alias(){
    setup_tir_builder!(tracker, r#"fn dup(x: str): str {return x;}fn foo(){let s = "hello"; let y = dup(s); println(y);} foo();"#);
    assert!(tracker.has_alias(1, "foo", 1), "{:#?}", tracker.encapsulators)
}

#[test]
fn test_returning_alias(){
    setup_tir_builder!(tracker, r#"fn dup(x: str): str{return x}; fn ret_hello(): str {return "hi";} let x = ret_hello(); let y = dup(x);"#);
    assert!(tracker.has_alias(1, "ret_hello", 2));
    assert!(tracker.has_alias(1, "user_main", 2));
}

#[test]
fn test_multi_func_alias(){
    setup_tir_builder!(tracker, r#"fn f(x: str): str{return x}; fn g(x: str): str{return x}; let h = "hi"; let z = f(g(h));"#);
    assert!(tracker.has_alias(3, "user_main", 5), "{:#?}", tracker.aliases)
}

#[test]
fn test_struct_array_alias(){
    setup_tir_builder!(tracker, r#"
        let arr = ["hi", "bye"];
        struct S{a: str[]};
        let s = S{a: arr};
        println(s.a);
    "#);
    assert!(tracker.has_encapsulator(1, "user_main", 10), "{:#?}", tracker.encapsulators);
    assert!(tracker.has_encapsulator(3, "user_main", 13), "{:#?}", tracker.encapsulators);
    assert!(tracker.has_encapsulator(7, "user_main", 15), "{:#?}", tracker.encapsulators);
}