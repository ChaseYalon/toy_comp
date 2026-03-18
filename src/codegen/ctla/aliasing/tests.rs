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