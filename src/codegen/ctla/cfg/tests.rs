use crate::codegen::AstToIrConverter;
use crate::codegen::ctla::cfg::CFGFunction;
use crate::codegen::tir::ir::{BlockId, Function};
use crate::driver::Driver;
use crate::parser::ast::Ast;
use crate::parser::ast_gen::AstGenerator;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

fn parse_test_code(code: impl ToString) -> (Vec<Ast>, Driver) {
    let mut driver = Driver::new(PathBuf::from("temp/test.toy"));
    let mut ast_gen = AstGenerator::new();
    let ast = driver
        .compile_to_ast_from_str(code.to_string(), &mut ast_gen)
        .unwrap();
    (ast, driver)
}

macro_rules! setup_tir {
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
                    t.builder
                        .register_extern_func(full_mangled, ret.clone(), true, vec![]);
                }
            }
        }
        let $o = t.convert(ast, true, "test").unwrap();
    };
}

fn get_main_function(funcs: &[Function]) -> Function {
    return funcs
        .iter()
        .find(|f| f.name.as_ref() == "user_main")
        .cloned()
        .unwrap_or_else(|| funcs[0].clone());
}

fn build_cfg(func: Function) -> CFGFunction {
    let mut cfg = CFGFunction::new(func);
    cfg.calc_cfg();
    return cfg;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedBlock {
    id: BlockId,
    inputs: Vec<BlockId>,
    outputs: Vec<BlockId>,
}

fn normalize_cfg(cfg: &CFGFunction) -> Vec<ExpectedBlock> {
    let mut actual_ids: Vec<BlockId> = cfg.cfg_blocks.iter().map(|b| b.block).collect();
    actual_ids.sort_unstable();

    let id_map: HashMap<BlockId, BlockId> = actual_ids
        .iter()
        .enumerate()
        .map(|(idx, old_id)| (*old_id, idx as BlockId))
        .collect();

    let mut by_norm_id: BTreeMap<BlockId, ExpectedBlock> = BTreeMap::new();

    for block in &cfg.cfg_blocks {
        let norm_id = *id_map.get(&block.block).unwrap();
        let mut inputs: Vec<BlockId> = block
            .possible_input_blocks
            .iter()
            .map(|id| *id_map.get(id).unwrap())
            .collect();
        let mut outputs: Vec<BlockId> = block
            .possible_output_blocks
            .iter()
            .map(|id| *id_map.get(id).unwrap())
            .collect();

        inputs.sort_unstable();
        outputs.sort_unstable();

        by_norm_id.insert(
            norm_id,
            ExpectedBlock {
                id: norm_id,
                inputs,
                outputs,
            },
        );
    }

    by_norm_id.into_values().collect()
}

fn assert_cfg_matches(code: &str, expected: &[ExpectedBlock]) {
    setup_tir!(ir, code);
    let cfg = build_cfg(get_main_function(&ir));
    let got = normalize_cfg(&cfg);
    assert_eq!(got, expected, "CFG mismatch for source: {}", code);
}

#[test]
fn test_cfg_literal_hardcoded() {
    assert_cfg_matches(
        "5",
        &[ExpectedBlock {
            id: 0,
            inputs: vec![],
            outputs: vec![],
        }],
    );
}

#[test]
fn test_cfg_if_else_hardcoded() {
    assert_cfg_matches(
        "if true { println(1); } else { println(2); }",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1, 2],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![0],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1, 2],
                outputs: vec![],
            },
        ],
    );
}

#[test]
fn test_cfg_if_else_merge_hardcoded() {
    assert_cfg_matches(
        "let x = 0; if true { x = 1; } else { x = 2; } println(x);",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1, 2],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![0],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1, 2],
                outputs: vec![],
            },
        ],
    );
}

#[test]
fn test_cfg_while_loop_hardcoded() {
    assert_cfg_matches(
        "let i = 0; while i < 3 { i += 1; }",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0, 2],
                outputs: vec![2, 3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![1],
                outputs: vec![1],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1],
                outputs: vec![],
            },
        ],
    );
}

#[test]
fn test_cfg_nested_if_hardcoded() {
    assert_cfg_matches(
        "let a = 1; if a == 1 { if true { println(1); } else { println(2); } } else { println(3); }",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1, 2],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0],
                outputs: vec![3, 4],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![0],
                outputs: vec![6],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1],
                outputs: vec![5],
            },
            ExpectedBlock {
                id: 4,
                inputs: vec![1],
                outputs: vec![5],
            },
            ExpectedBlock {
                id: 5,
                inputs: vec![3, 4],
                outputs: vec![6],
            },
            ExpectedBlock {
                id: 6,
                inputs: vec![2, 5],
                outputs: vec![],
            },
        ],
    );
}

#[test]
fn test_cfg_break_continue_hardcoded() {
    assert_cfg_matches(
        "let i = 0; while i < 5 { if i == 2 { i += 1; continue; } if i == 4 { break; } i += 1; }",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0, 4],
                outputs: vec![2, 3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![1],
                outputs: vec![5, 6],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1, 7],
                outputs: vec![],
            },
            ExpectedBlock {
                id: 4,
                inputs: vec![5, 8],
                outputs: vec![1],
            },
            ExpectedBlock {
                id: 5,
                inputs: vec![2],
                outputs: vec![4],
            },
            ExpectedBlock {
                id: 6,
                inputs: vec![2],
                outputs: vec![7, 8],
            },
            ExpectedBlock {
                id: 7,
                inputs: vec![6],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 8,
                inputs: vec![6],
                outputs: vec![4],
            },
        ],
    );
}

#[test]
fn test_cfg_function_call_hardcoded() {
    assert_cfg_matches(
        "fn inc(n: int): int { return n + 1; } let x = inc(1); println(x);",
        &[ExpectedBlock {
            id: 0,
            inputs: vec![],
            outputs: vec![],
        }],
    );
}

#[test]
fn test_cfg_branch_and_loop_hardcoded() {
    assert_cfg_matches(
        "let i = 0; while i < 3 { if i == 1 { println(10); } else { println(20); } i += 1; }",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0, 6],
                outputs: vec![2, 3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![1],
                outputs: vec![4, 5],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1],
                outputs: vec![],
            },
            ExpectedBlock {
                id: 4,
                inputs: vec![2],
                outputs: vec![6],
            },
            ExpectedBlock {
                id: 5,
                inputs: vec![2],
                outputs: vec![6],
            },
            ExpectedBlock {
                id: 6,
                inputs: vec![4, 5],
                outputs: vec![1],
            },
        ],
    );
}

#[test]
fn test_cfg_two_ifs_hardcoded() {
    assert_cfg_matches(
        "let x = 0; if x == 0 { x = 1; } else { x = 2; } if x == 2 { println(2); } println(x);",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1, 2],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![0],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1, 2],
                outputs: vec![4, 5],
            },
            ExpectedBlock {
                id: 4,
                inputs: vec![3],
                outputs: vec![5],
            },
            ExpectedBlock {
                id: 5,
                inputs: vec![3, 4],
                outputs: vec![],
            },
        ],
    );
}

#[test]
fn test_cfg_if_elseif_else_hardcoded() {
    assert_cfg_matches(
        "let x = 1; if x == 0 { println(0); } else if x == 1 { println(1); } else { println(2); }",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1, 2],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0],
                outputs: vec![3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![0],
                outputs: vec![4, 5],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1, 6],
                outputs: vec![],
            },
            ExpectedBlock {
                id: 4,
                inputs: vec![2],
                outputs: vec![6],
            },
            ExpectedBlock {
                id: 5,
                inputs: vec![2],
                outputs: vec![6],
            },
            ExpectedBlock {
                id: 6,
                inputs: vec![4, 5],
                outputs: vec![3],
            },
        ],
    );
}

#[test]
fn test_cfg_nested_while_if() {
    assert_cfg_matches(
        "let i = 0; while i < 10 { if i % 2 == 0 {} i += 1; }",
        &[
            ExpectedBlock {
                id: 0,
                inputs: vec![],
                outputs: vec![1],
            },
            ExpectedBlock {
                id: 1,
                inputs: vec![0, 5],
                outputs: vec![2, 3],
            },
            ExpectedBlock {
                id: 2,
                inputs: vec![1],
                outputs: vec![4, 5],
            },
            ExpectedBlock {
                id: 3,
                inputs: vec![1],
                outputs: vec![],
            },
            ExpectedBlock {
                id: 4,
                inputs: vec![2],
                outputs: vec![5],
            },
            ExpectedBlock {
                id: 5,
                inputs: vec![2, 4],
                outputs: vec![1],
            },
        ],
    );
}
