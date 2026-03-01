use crate::codegen::AstToIrConverter;
use crate::codegen::tir::ir::{
    Block, BoolInfixOp, Function, NumericInfixOp, SSAValue, TIR, TirType,
};
use crate::driver::Driver;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::Ast;
use crate::parser::ast_gen::AstGenerator;
use colored::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, fs};

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
                    t.builder.register_extern_func(full_mangled, ret.clone());
                }
            }
        }
        let $o = t.convert(ast, true, "test").unwrap();
    };
}
fn panic_with_write(test_name: &str, a: Vec<Function>, b: Vec<Function>) {
    let file = Path::new(env::var("CARGO_MANIFEST_DIR").unwrap().as_str())
        .join(Path::new("temp"))
        .join(format!("tir_test_{}.txt", test_name));
    let mut f = File::create(file.clone()).unwrap();
    fs::write(
        file,
        format!("GOT: \n{:#?}\n\nWANT:\n{:#?}", a, b).as_bytes(),
    )
    .unwrap();
    panic!();
}
fn compare_tir(test_name: &str, a: Vec<Function>, b: Vec<Function>) {
    if a.len() != b.len() {
        panic!(
            "{}\nGenerated: {} functions, got {} functions",
            "[TEST ERROR]"
                .color(Color::TrueColor {
                    r: 255,
                    g: 147,
                    b: 32
                })
                .bold(),
            a.len(),
            b.len()
        );
    }

    for (i, func) in a.iter().enumerate() {
        let Function {
            name: g_name,
            ret_type: g_ret_type,
            params: g_params,
            body: g_body,
            ..
        } = func.clone();
        let Function {
            name: r_name,
            ret_type: r_ret_type,
            params: r_params,
            body: r_body,
            ..
        } = b[i].clone();

        if g_name != r_name {
            eprintln!(
                "{}\nExpected function name: {}, got: {}",
                "[TEST ERROR]"
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold(),
                r_name.blue().bold(),
                g_name
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold()
            );
            panic_with_write(test_name, a.clone(), b.clone());
        }

        if g_ret_type != r_ret_type {
            eprintln!(
                "{}\nIn function {}, expected return type: {}, got: {}",
                "[TEST ERROR]"
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold(),
                g_name.blue().bold(),
                r_ret_type.to_string().green().bold(),
                g_ret_type
                    .to_string()
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold()
            );
            panic_with_write(test_name, a.clone(), b.clone());
        }

        if g_params.len() != r_params.len() {
            eprintln!(
                "{}\nIn function {}, expected {} params, got: {} params",
                "[TEST ERROR]"
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold(),
                g_name.blue().bold(),
                r_params.len().to_string().green().bold(),
                g_params
                    .len()
                    .to_string()
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold()
            );
            panic_with_write(test_name, a.clone(), b.clone());
        }

        for (j, g_param) in g_params.iter().enumerate() {
            let r_param = &r_params[j];
            if g_param != r_param {
                eprintln!(
                    "{}\nIn function {}, param {}: expected {}, got: {}",
                    "[TEST ERROR]"
                        .color(Color::TrueColor {
                            r: 255,
                            g: 147,
                            b: 32
                        })
                        .bold(),
                    g_name.blue().bold(),
                    j.to_string().blue().bold(),
                    format!("{:#?}", r_param).green().bold(),
                    format!("{:#?}", g_param)
                        .color(Color::TrueColor {
                            r: 255,
                            g: 147,
                            b: 32
                        })
                        .bold()
                );
                panic_with_write(test_name, a.clone(), b.clone());
            }
        }

        if g_body.len() != r_body.len() {
            eprintln!(
                "{}\nIn function {}, expected {} blocks, got: {} blocks",
                "[TEST ERROR]"
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold(),
                g_name.blue().bold(),
                r_body.len().to_string().green().bold(),
                g_body
                    .len()
                    .to_string()
                    .color(Color::TrueColor {
                        r: 255,
                        g: 147,
                        b: 32
                    })
                    .bold()
            );
            panic_with_write(test_name, a.clone(), b.clone());
        }

        for (j, g_block) in g_body.iter().enumerate() {
            let r_block = &r_body[j];
            if g_block.ins.len() != r_block.ins.len() {
                eprintln!(
                    "{}\nIn function {}, {} {}, expected {} instructions, got: {} instructions",
                    "[TEST ERROR]"
                        .color(Color::TrueColor {
                            r: 255,
                            g: 147,
                            b: 32
                        })
                        .bold(),
                    g_name.blue().bold(),
                    "block".blue(),
                    j.to_string().blue().bold(),
                    r_block.ins.len().to_string().green().bold(),
                    g_block.ins.len().to_string().red().bold()
                );
                panic_with_write(test_name, a.clone(), b.clone());
            }

            for (k, g_ins) in g_block.ins.iter().enumerate() {
                let r_ins = &r_block.ins[k];
                if g_ins != r_ins {
                    let temp = format!("{:#?}", r_ins);
                    let wanted = temp.lines().collect::<Vec<_>>();
                    let temp = format!("{:#?}", g_ins);
                    let got = temp.lines().collect::<Vec<_>>();
                    let max_lines = wanted.len().max(got.len());
                    let col_width = 30; // width of the WANTED column

                    eprintln!(
                        "{}\n{} {}, {} {}, {} {}",
                        "[TEST ERROR]"
                            .color(Color::TrueColor {
                                r: 255,
                                g: 147,
                                b: 32
                            })
                            .bold(),
                        "function".blue(),
                        g_name.blue().bold(),
                        "block".blue(),
                        j.to_string().blue().bold(),
                        "instruction".blue(),
                        k.to_string().blue().bold()
                    );

                    let half_width = col_width / 2;
                    // Column headers
                    eprintln!(
                        "{:<width$} │ {:<width$}",
                        "WANTED".green().bold(),
                        "GOT".red().bold(),
                        width = col_width
                    );
                    eprintln!("{:<width$} │ {:<width$}", "", "", width = col_width);
                    // Now the lines for WANTED / GOT
                    for i in 0..max_lines {
                        let w = wanted.get(i).unwrap_or(&"");
                        let g = got.get(i).unwrap_or(&"");
                        eprintln!(
                            "{:<width$} │ {}",
                            w.green().bold(),
                            g.red().bold(),
                            width = col_width
                        );
                    }

                    panic_with_write(test_name, a.clone(), b.clone());
                }
            }
        }
    }
}

#[test]
fn test_tirgen_int_lit() {
    setup_tir!(ir, "5");
    compare_tir(
        "int_lit",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 5, TirType::I64),
                    TIR::IConst(1, 0, TirType::I64),
                    TIR::Ret(
                        2,
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 3,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}
#[test]
fn test_tirgen_bool_lit() {
    setup_tir!(ir, "true");
    compare_tir(
        "bool_lit",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I1),
                    TIR::IConst(1, 0, TirType::I64),
                    TIR::Ret(
                        2,
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 3,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}
#[test]
fn test_tirgen_numeric_infix() {
    setup_tir!(ir, "5 + 3 * 9");
    compare_tir(
        "numeric_infix",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 5, TirType::I64),
                    TIR::IConst(1, 3, TirType::I64),
                    TIR::IConst(2, 9, TirType::I64),
                    TIR::NumericInfix(
                        3,
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Multiply,
                    ),
                    TIR::NumericInfix(
                        4,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    TIR::IConst(5, 0, TirType::I64),
                    TIR::Ret(
                        6,
                        SSAValue {
                            val: 5,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 7,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}
#[test]
fn test_tirgen_boolean_infix() {
    setup_tir!(ir, "true && false");
    compare_tir(
        "boolean_infix",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I1),
                    TIR::IConst(1, 0, TirType::I1),
                    TIR::BoolInfix(
                        2,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I1),
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I1),
                        },
                        BoolInfixOp::And,
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 5,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_var_dec_and_reassign() {
    setup_tir!(ir, "let x = 9; x += 3");
    compare_tir(
        "var_dec_and_reassign",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 9, TirType::I64),
                    TIR::IConst(1, 3, TirType::I64),
                    TIR::NumericInfix(
                        2,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 5,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_var_ref() {
    setup_tir!(ir, "let x = 9; x + 4");
    compare_tir(
        "var_ref",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 9, TirType::I64),
                    TIR::IConst(1, 4, TirType::I64),
                    TIR::NumericInfix(
                        2,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 5,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_if_stmt() {
    setup_tir!(ir, "let x = true || false; if x {5}; 9 + 3;");
    compare_tir(
        "if_stmt",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![
                Block {
                    id: 0,
                    ins: vec![
                        TIR::IConst(0, 1, TirType::I1),
                        TIR::IConst(1, 0, TirType::I1),
                        TIR::BoolInfix(
                            2,
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I1),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I1),
                            },
                            BoolInfixOp::Or,
                        ),
                        TIR::JumpCond(
                            3,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I1),
                            },
                            1,
                            2,
                        ),
                    ],
                },
                Block {
                    id: 1,
                    ins: vec![TIR::IConst(4, 5, TirType::I64), TIR::JumpBlockUnCond(5, 2)],
                },
                Block {
                    id: 2,
                    ins: vec![
                        TIR::IConst(6, 9, TirType::I64),
                        TIR::IConst(7, 3, TirType::I64),
                        TIR::NumericInfix(
                            8,
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64),
                            },
                            NumericInfixOp::Plus,
                        ),
                        TIR::IConst(9, 0, TirType::I64),
                        TIR::Ret(
                            10,
                            SSAValue {
                                val: 9,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                },
            ],
            ins_counter: 11,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_if_else_stmt() {
    setup_tir!(ir, "let x = true || false; if x {5} else {9 + 3};");
    compare_tir(
        "if_else_stmt",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![
                Block {
                    id: 0,
                    ins: vec![
                        TIR::IConst(0, 1, TirType::I1),
                        TIR::IConst(1, 0, TirType::I1),
                        TIR::BoolInfix(
                            2,
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I1),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I1),
                            },
                            BoolInfixOp::Or,
                        ),
                        TIR::JumpCond(
                            3,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I1),
                            },
                            1,
                            2,
                        ),
                    ],
                },
                Block {
                    id: 1,
                    ins: vec![TIR::IConst(4, 5, TirType::I64), TIR::JumpBlockUnCond(5, 3)],
                },
                Block {
                    id: 2,
                    ins: vec![
                        TIR::IConst(6, 9, TirType::I64),
                        TIR::IConst(7, 3, TirType::I64),
                        TIR::NumericInfix(
                            8,
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64),
                            },
                            NumericInfixOp::Plus,
                        ),
                        TIR::JumpBlockUnCond(9, 3),
                    ],
                },
                Block {
                    id: 3,
                    ins: vec![
                        TIR::IConst(10, 0, TirType::I64),
                        TIR::Ret(
                            11,
                            SSAValue {
                                val: 10,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                },
            ],
            ins_counter: 12,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_empty_expr() {
    setup_tir!(ir, "let x = 9 * (4 + 3)");
    compare_tir(
        "empty_expr",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 9, TirType::I64),
                    TIR::IConst(1, 4, TirType::I64),
                    TIR::IConst(2, 3, TirType::I64),
                    TIR::NumericInfix(
                        3,
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    TIR::NumericInfix(
                        4,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Multiply,
                    ),
                    TIR::IConst(5, 0, TirType::I64),
                    TIR::Ret(
                        6,
                        SSAValue {
                            val: 5,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 7,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_func_call() {
    setup_tir!(
        ir,
        "fn add(a: int, b: int): int { return a + b }; add(3, 5)"
    );
    compare_tir(
        "func_call",
        ir,
        vec![
            Function {
                params: vec![],
                name: Box::new("user_main".to_string()),
                body: vec![Block {
                    id: 0,
                    ins: vec![
                        TIR::IConst(2, 3, TirType::I64),
                        TIR::IConst(3, 5, TirType::I64),
                        TIR::CallLocalFunction(
                            4,
                            Box::new("add_int_int".to_string()),
                            vec![
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 3,
                                    ty: Some(TirType::I64),
                                },
                            ],
                            false,
                            TirType::I64,
                        ),
                        TIR::IConst(5, 0, TirType::I64),
                        TIR::Ret(
                            6,
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                }],
                ins_counter: 7,
                ret_type: TirType::I64,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            // add functionK
            Function {
                params: vec![
                    SSAValue {
                        val: 0,
                        ty: Some(TirType::I64),
                    },
                    SSAValue {
                        val: 1,
                        ty: Some(TirType::I64),
                    },
                ],
                name: Box::new("add_int_int".to_string()),
                body: vec![Block {
                    id: 1,
                    ins: vec![
                        TIR::NumericInfix(
                            2,
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            NumericInfixOp::Plus,
                        ),
                        TIR::Ret(
                            3,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                }],
                ins_counter: 4,
                ret_type: TirType::I64,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    )
}

#[test]
fn test_tirgen_while_stmt() {
    // Simple while loop test: while 5 < 10 { } - condition is always true, so loop body executes once
    setup_tir!(ir, "while 5 < 10 { }");
    compare_tir(
        "while_stmt",
        ir,
        vec![Function {
            params: vec![],
            body: vec![
                Block {
                    id: 0,
                    ins: vec![TIR::JumpBlockUnCond(0, 1)],
                },
                Block {
                    id: 1,
                    ins: vec![
                        TIR::IConst(1, 5, TirType::I64),
                        TIR::IConst(2, 10, TirType::I64),
                        TIR::BoolInfix(
                            3,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                            BoolInfixOp::LessThan,
                        ),
                        TIR::JumpCond(
                            4,
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::I1),
                            },
                            2,
                            3,
                        ),
                    ],
                },
                Block {
                    id: 2,
                    ins: vec![TIR::JumpBlockUnCond(5, 1)],
                },
                Block {
                    id: 3,
                    ins: vec![
                        TIR::IConst(6, 0, TirType::I64),
                        TIR::Ret(
                            7,
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                },
            ],
            name: Box::new("user_main".to_string()),
            ret_type: TirType::I64,
            ins_counter: 8,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_while_with_var_mod() {
    setup_tir!(ir, "let x = 0; while x < 3 { x = x + 1 }");
    compare_tir(
        "while_with_var_mod",
        ir,
        vec![Function {
            params: vec![],
            body: vec![
                Block {
                    id: 0,
                    ins: vec![TIR::IConst(0, 0, TirType::I64), TIR::JumpBlockUnCond(1, 1)],
                },
                Block {
                    id: 1,
                    ins: vec![
                        TIR::Phi(
                            2,
                            vec![0, 2],
                            vec![
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::I64),
                                },
                            ],
                        ),
                        TIR::IConst(3, 3, TirType::I64),
                        TIR::BoolInfix(
                            4,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::I64),
                            },
                            BoolInfixOp::LessThan,
                        ),
                        TIR::JumpCond(
                            5,
                            SSAValue {
                                val: 4,
                                ty: Some(TirType::I1),
                            },
                            2,
                            3,
                        ),
                    ],
                },
                Block {
                    id: 2,
                    ins: vec![
                        TIR::IConst(6, 1, TirType::I64),
                        TIR::NumericInfix(
                            7,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                            NumericInfixOp::Plus,
                        ),
                        TIR::JumpBlockUnCond(8, 1),
                    ],
                },
                Block {
                    id: 3,
                    ins: vec![
                        TIR::IConst(9, 0, TirType::I64),
                        TIR::Ret(
                            10,
                            SSAValue {
                                val: 9,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                },
            ],
            name: Box::new("user_main".to_string()),
            ret_type: TirType::I64,
            ins_counter: 11,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_string_lit_concat_and_equals() {
    setup_tir!(
        ir,
        r#"let x = "foo"; let y = "fee"; let z = x + y; let a = x == y;"#
    );
    compare_tir(
        "string_lit_concat_and_equals",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            ret_type: TirType::I64,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::GlobalString(0, Box::new("foo".to_string())),
                    TIR::CallExternFunction(
                        1,
                        Box::new("toy_malloc".to_string()),
                        vec![SSAValue {
                            val: 0,
                            ty: Some(TirType::Ptr),
                        }],
                        true,
                        TirType::Ptr,
                    ),
                    TIR::GlobalString(2, Box::new("fee".to_string())),
                    TIR::CallExternFunction(
                        3,
                        Box::new("toy_malloc".to_string()),
                        vec![SSAValue {
                            val: 2,
                            ty: Some(TirType::Ptr),
                        }],
                        true,
                        TirType::Ptr,
                    ),
                    TIR::CallExternFunction(
                        4,
                        Box::new("toy_concat".to_string()),
                        vec![
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::Ptr),
                            },
                        ],
                        true,
                        TirType::Ptr,
                    ),
                    TIR::CallExternFunction(
                        5,
                        Box::new("toy_strequal".to_string()),
                        vec![
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::Ptr),
                            },
                        ],
                        false,
                        TirType::I64,
                    ),
                    TIR::IConst(6, 0, TirType::I64),
                    TIR::Ret(
                        7,
                        SSAValue {
                            val: 6,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 8,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_float_lit_and_opps() {
    setup_tir!(ir, "let x = 9.2 + 6");
    compare_tir(
        "float_lit_and_opps",
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::FConst(0, 9.2, TirType::F64),
                    TIR::IConst(1, 6, TirType::I64),
                    TIR::ItoF(
                        2,
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        TirType::F64,
                    ),
                    TIR::NumericInfix(
                        3,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::F64),
                        },
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::F64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    TIR::IConst(4, 0, TirType::I64),
                    TIR::Ret(
                        5,
                        SSAValue {
                            val: 4,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ret_type: TirType::I64,
            ins_counter: 6,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_arr_lit_read_and_write() {
    setup_tir!(ir, "let arr = [1, 2, 3]; arr[2] = 9; let x = arr[1] + 3;");
    compare_tir(
        "arr_lit_read_and_write",
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            ret_type: TirType::I64,
            ins_counter: 26,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I64),
                    TIR::IConst(1, 2, TirType::I64),
                    TIR::IConst(2, 3, TirType::I64),
                    TIR::IConst(3, 3, TirType::I64),
                    TIR::IConst(4, 2, TirType::I64),
                    TIR::IConst(5, 1, TirType::I64),
                    TIR::CallExternFunction(
                        6,
                        Box::new("toy_malloc_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 4,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                        ],
                        true,
                        TirType::Ptr,
                    ),
                    TIR::IConst(7, 0, TirType::I64),
                    TIR::IConst(8, 2, TirType::I64),
                    TIR::CallExternFunction(
                        9,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 8,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(10, 1, TirType::I64),
                    TIR::IConst(11, 2, TirType::I64),
                    TIR::CallExternFunction(
                        12,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 10,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 11,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(13, 2, TirType::I64),
                    TIR::IConst(14, 2, TirType::I64),
                    TIR::CallExternFunction(
                        15,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 13,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 14,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(16, 9, TirType::I64),
                    TIR::IConst(17, 2, TirType::I64),
                    TIR::IConst(18, 2, TirType::I64),
                    TIR::CallExternFunction(
                        19,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 16,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 17,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 18,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(20, 1, TirType::I64),
                    TIR::CallExternFunction(
                        21,
                        Box::new("toy_read_from_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 20,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::I64,
                    ),
                    TIR::IConst(22, 3, TirType::I64),
                    TIR::NumericInfix(
                        23,
                        SSAValue {
                            val: 21,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 22,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    // Implicit return 0
                    TIR::IConst(24, 0, TirType::I64),
                    TIR::Ret(
                        25,
                        SSAValue {
                            val: 24,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_struct_lit() {
    setup_tir!(
        ir,
        "struct Point{x: float, y: float}; let origin = Point{x: 0.0, y: 0.0}; let x = origin.x; origin.y = 3.4;"
    );
    compare_tir(
        "struct_lit",
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            ret_type: TirType::I64,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::CreateStructInterface(
                        0,
                        Box::new("Point".to_string()),
                        TirType::StructInterface(vec![TirType::F64, TirType::F64]),
                    ),
                    TIR::FConst(1, 0.0, TirType::F64),
                    TIR::FConst(2, 0.0, TirType::F64),
                    TIR::CreateStructLiteral(
                        3,
                        TirType::StructInterface(vec![TirType::F64, TirType::F64]),
                        vec![
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::F64),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::F64),
                            },
                        ],
                    ),
                    TIR::ReadStructLiteral(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::StructInterface(vec![TirType::F64, TirType::F64])),
                        },
                        0,
                    ),
                    TIR::FConst(5, 3.4, TirType::F64),
                    TIR::WriteStructLiteral(
                        6,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::StructInterface(vec![TirType::F64, TirType::F64])),
                        },
                        1,
                        SSAValue {
                            val: 5,
                            ty: Some(TirType::F64),
                        },
                    ),
                    TIR::IConst(7, 0, TirType::I64),
                    TIR::Ret(
                        8,
                        SSAValue {
                            val: 7,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 9,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_not() {
    setup_tir!(ir, "let x = false; let y = !x;");
    compare_tir(
        "not",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 0, TirType::I1),
                    TIR::Not(
                        1,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I1),
                        },
                    ),
                    TIR::IConst(2, 0, TirType::I64),
                    TIR::Ret(
                        3,
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 4,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

//why do the ids start with 1?
#[test]
fn test_tirgen_recursion_bug() {
    setup_tir!(
        ir,
        "
    fn fib(n: int): int{
        if n == 0 {
            return 0;
        }
        if n == 1 {
            return 1;
        }
        return fib(n - 1) + fib(n - 2);
    }
    println(fib(40));"
    );
    compare_tir(
        "recursion_bug",
        ir,
        vec![
            Function {
                name: Box::new("user_main".to_string()),
                params: vec![],
                ret_type: TirType::I64,
                body: vec![Block {
                    id: 0,
                    ins: vec![
                        TIR::IConst(1, 40, TirType::I64),
                        TIR::CallLocalFunction(
                            2,
                            Box::new("fib_int".to_string()),
                            vec![SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            }],
                            false,
                            TirType::I64,
                        ),
                        TIR::IConst(3, 2, TirType::I64),
                        TIR::IConst(4, 0, TirType::I64),
                        TIR::CallExternFunction(
                            5,
                            Box::new("toy_println".to_string()),
                            vec![
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 3,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 4,
                                    ty: Some(TirType::I64),
                                },
                            ],
                            false,
                            TirType::Void,
                        ),
                        TIR::IConst(6, 0, TirType::I64),
                        TIR::Ret(
                            7,
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                }],
                ins_counter: 8,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            Function {
                name: Box::new("fib_int".to_string()),
                params: vec![SSAValue {
                    val: 0,
                    ty: Some(TirType::I64),
                }],
                ret_type: TirType::I64,
                body: vec![
                    Block {
                        id: 1,
                        ins: vec![
                            TIR::IConst(1, 0, TirType::I64),
                            TIR::BoolInfix(
                                2,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 1,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::Equals,
                            ),
                            TIR::JumpCond(
                                3,
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I1),
                                },
                                2,
                                3,
                            ),
                        ],
                    },
                    Block {
                        id: 2,
                        ins: vec![
                            TIR::IConst(4, 0, TirType::I64),
                            TIR::Ret(
                                5,
                                SSAValue {
                                    val: 4,
                                    ty: Some(TirType::I64),
                                },
                            ),
                        ],
                    },
                    Block {
                        id: 3,
                        ins: vec![
                            TIR::IConst(6, 1, TirType::I64),
                            TIR::BoolInfix(
                                7,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 6,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::Equals,
                            ),
                            TIR::JumpCond(
                                8,
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::I1),
                                },
                                4,
                                5,
                            ),
                        ],
                    },
                    Block {
                        id: 4,
                        ins: vec![
                            TIR::IConst(9, 1, TirType::I64),
                            TIR::Ret(
                                10,
                                SSAValue {
                                    val: 9,
                                    ty: Some(TirType::I64),
                                },
                            ),
                        ],
                    },
                    Block {
                        id: 5,
                        ins: vec![
                            TIR::IConst(11, 1, TirType::I64),
                            TIR::NumericInfix(
                                12,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 11,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Minus,
                            ),
                            TIR::CallLocalFunction(
                                13,
                                Box::new("fib_int".to_string()),
                                vec![SSAValue {
                                    val: 12,
                                    ty: Some(TirType::I64),
                                }],
                                false,
                                TirType::I64,
                            ),
                            TIR::IConst(14, 2, TirType::I64),
                            TIR::NumericInfix(
                                15,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 14,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Minus,
                            ),
                            TIR::CallLocalFunction(
                                16,
                                Box::new("fib_int".to_string()),
                                vec![SSAValue {
                                    val: 15,
                                    ty: Some(TirType::I64),
                                }],
                                false,
                                TirType::I64,
                            ),
                            TIR::NumericInfix(
                                17,
                                SSAValue {
                                    val: 13,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 16,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Plus,
                            ),
                            TIR::Ret(
                                18,
                                SSAValue {
                                    val: 17,
                                    ty: Some(TirType::I64),
                                },
                            ),
                        ],
                    },
                ],
                ins_counter: 19,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    )
}

#[test]
fn test_tirgen_broken_booleans() {
    setup_tir!(ir, "let x = true || false; println(!x);");
    compare_tir(
        "broken_booleans",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I1),
                    TIR::IConst(1, 0, TirType::I1),
                    TIR::BoolInfix(
                        2,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I1),
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I1),
                        },
                        BoolInfixOp::Or,
                    ),
                    TIR::Not(
                        3,
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I1),
                        },
                    ),
                    TIR::IConst(4, 1, TirType::I64),
                    TIR::IConst(5, 0, TirType::I64),
                    TIR::CallExternFunction(
                        6,
                        Box::new("toy_println".to_string()),
                        vec![
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::I1),
                            },
                            SSAValue {
                                val: 4,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(7, 0, TirType::I64),
                    TIR::Ret(
                        8,
                        SSAValue {
                            val: 7,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 9,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}
//TODO: Nested arrays

#[test]
fn test_tirgen_broken_floats() {
    setup_tir!(ir, "println(5.32);");
    compare_tir(
        "broken_floats",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::FConst(0, 5.32, TirType::F64),
                    TIR::IConst(1, 3, TirType::I64),
                    TIR::IConst(2, 0, TirType::I64),
                    TIR::CallExternFunction(
                        3,
                        Box::new("toy_println".to_string()),
                        vec![
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::F64),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(4, 0, TirType::I64),
                    TIR::Ret(
                        5,
                        SSAValue {
                            val: 4,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ret_type: TirType::I64,
            ins_counter: 6,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    )
}

#[test]
fn test_tirgen_print_arr_lit() {
    setup_tir!(ir, "let arr = [1, 2, 3]; println(arr)");
    compare_tir(
        "print_arr_lit",
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            ret_type: TirType::I64,
            ins_counter: 21,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I64),
                    TIR::IConst(1, 2, TirType::I64),
                    TIR::IConst(2, 3, TirType::I64),
                    TIR::IConst(3, 3, TirType::I64),
                    TIR::IConst(4, 2, TirType::I64),
                    TIR::IConst(5, 1, TirType::I64),
                    TIR::CallExternFunction(
                        6,
                        Box::new("toy_malloc_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 4,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                        ],
                        true,
                        TirType::Ptr,
                    ),
                    TIR::IConst(7, 0, TirType::I64),
                    TIR::IConst(8, 2, TirType::I64),
                    TIR::CallExternFunction(
                        9,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 8,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(10, 1, TirType::I64),
                    TIR::IConst(11, 2, TirType::I64),
                    TIR::CallExternFunction(
                        12,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 10,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 11,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(13, 2, TirType::I64),
                    TIR::IConst(14, 2, TirType::I64),
                    TIR::CallExternFunction(
                        15,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 13,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 14,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(16, 6, TirType::I64),
                    TIR::IConst(17, 1, TirType::I64),
                    TIR::CallExternFunction(
                        18,
                        Box::new("toy_println".to_string()),
                        vec![
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 16,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 17,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(19, 0, TirType::I64),
                    TIR::Ret(
                        20,
                        SSAValue {
                            val: 19,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    );
}

#[test]
fn test_tirgen_if_no_else_return() {
    setup_tir!(
        ir,
        r#"fn isEven(n: int): str {if n % 2 == 0 {return "it is";} return "it is not";} println(isEven(5));"#
    );
    compare_tir(
        "if_no_else_return",
        ir,
        vec![
            Function {
                params: vec![],
                name: Box::new("user_main".to_string()),
                body: vec![Block {
                    id: 0,
                    ins: vec![
                        TIR::IConst(1, 5, TirType::I64),
                        TIR::CallLocalFunction(
                            2,
                            Box::new("isEven_int".to_string()),
                            vec![SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            }],
                            true,
                            TirType::Ptr,
                        ),
                        TIR::IConst(3, 0, TirType::I64),
                        TIR::IConst(4, 0, TirType::I64),
                        TIR::CallExternFunction(
                            5,
                            Box::new("toy_println".to_string()),
                            vec![
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::Ptr),
                                },
                                SSAValue {
                                    val: 3,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 4,
                                    ty: Some(TirType::I64),
                                },
                            ],
                            false,
                            TirType::Void,
                        ),
                        TIR::IConst(6, 0, TirType::I64),
                        TIR::Ret(
                            7,
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                }],
                ret_type: TirType::I64,
                ins_counter: 8,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            Function {
                params: vec![SSAValue {
                    val: 0,
                    ty: Some(TirType::I64),
                }],
                name: Box::new("isEven_int".to_string()),
                body: vec![
                    Block {
                        id: 1,
                        ins: vec![
                            TIR::IConst(1, 2, TirType::I64),
                            TIR::NumericInfix(
                                2,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 1,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Modulo,
                            ),
                            TIR::IConst(3, 0, TirType::I64),
                            TIR::BoolInfix(
                                4,
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 3,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::Equals,
                            ),
                            TIR::JumpCond(
                                5,
                                SSAValue {
                                    val: 4,
                                    ty: Some(TirType::I1),
                                },
                                2,
                                3,
                            ),
                        ],
                    },
                    Block {
                        id: 2,
                        ins: vec![
                            TIR::GlobalString(6, Box::new("it is".to_string())),
                            TIR::CallExternFunction(
                                7,
                                Box::new("toy_malloc".to_string()),
                                vec![SSAValue {
                                    val: 6,
                                    ty: Some(TirType::Ptr),
                                }],
                                true,
                                TirType::Ptr,
                            ),
                            TIR::Ret(
                                8,
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::Ptr),
                                },
                            ),
                        ],
                    },
                    Block {
                        id: 3,
                        ins: vec![
                            TIR::GlobalString(9, Box::new("it is not".to_string())),
                            TIR::CallExternFunction(
                                10,
                                Box::new("toy_malloc".to_string()),
                                vec![SSAValue {
                                    val: 9,
                                    ty: Some(TirType::Ptr),
                                }],
                                true,
                                TirType::Ptr,
                            ),
                            TIR::Ret(
                                11,
                                SSAValue {
                                    val: 10,
                                    ty: Some(TirType::Ptr),
                                },
                            ),
                        ],
                    },
                ],
                ret_type: TirType::Ptr,
                ins_counter: 12,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    );
}

#[test]
fn test_tirgen_struct_funcs() {
    setup_tir!(
        ir,
        "struct Point{x: int, y: int}; for Point { fn print_point() { println(this.x) } } let me = Point{x: 0, y: 0}; me.print_point();"
    );
    compare_tir(
        "struct_funcs",
        ir,
        vec![
            Function {
                params: vec![],
                name: Box::new("user_main".to_string()),
                body: vec![Block {
                    id: 0,
                    ins: vec![
                        TIR::CreateStructInterface(
                            0,
                            Box::new("Point".to_string()),
                            TirType::StructInterface(vec![TirType::I64, TirType::I64]),
                        ),
                        TIR::IConst(2, 0, TirType::I64),
                        TIR::IConst(3, 0, TirType::I64),
                        TIR::CreateStructLiteral(
                            4,
                            TirType::StructInterface(vec![TirType::I64, TirType::I64]),
                            vec![
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 3,
                                    ty: Some(TirType::I64),
                                },
                            ],
                        ),
                        TIR::CallLocalFunction(
                            5,
                            Box::new("Point:::print_point_struct".to_string()),
                            vec![SSAValue {
                                val: 4,
                                ty: Some(TirType::StructInterface(vec![
                                    TirType::I64,
                                    TirType::I64,
                                ])),
                            }],
                            false,
                            TirType::Void,
                        ),
                        TIR::IConst(6, 0, TirType::I64),
                        TIR::Ret(
                            7,
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                }],
                ins_counter: 8,
                ret_type: TirType::I64,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            Function {
                params: vec![SSAValue {
                    val: 1,
                    ty: Some(TirType::StructInterface(vec![TirType::I64, TirType::I64])),
                }],
                name: Box::new("Point:::print_point_struct".to_string()),
                body: vec![Block {
                    id: 1,
                    ins: vec![
                        TIR::ReadStructLiteral(
                            2,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::StructInterface(vec![
                                    TirType::I64,
                                    TirType::I64,
                                ])),
                            },
                            0,
                        ),
                        TIR::IConst(3, 2, TirType::I64),
                        TIR::IConst(4, 0, TirType::I64),
                        TIR::CallExternFunction(
                            5,
                            Box::new("toy_println".to_string()),
                            vec![
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 3,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 4,
                                    ty: Some(TirType::I64),
                                },
                            ],
                            false,
                            TirType::Void,
                        ),
                        TIR::Ret(6, SSAValue { val: 0, ty: None }),
                    ],
                }],
                ins_counter: 7,
                ret_type: TirType::Void,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    );
}

#[test]
fn test_tirgen_struct_func_multi() {
    setup_tir!(
        ir,
        r#"
struct Point{
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

println(points);
"#
    );
    compare_tir(
        "struct_func_multi",
        ir,
        vec![
            Function {
                params: vec![],
                name: Box::new("user_main".to_string()),
                body: vec![
                    Block {
                        id: 0,
                        ins: vec![
                            TIR::CreateStructInterface(
                                0,
                                Box::new("Point".to_string()),
                                TirType::StructInterface(vec![TirType::F64, TirType::F64]),
                            ),
                            TIR::FConst(4, 0.0, TirType::F64),
                            TIR::FConst(5, 0.0, TirType::F64),
                            TIR::CreateStructLiteral(
                                6,
                                TirType::StructInterface(vec![TirType::F64, TirType::F64]),
                                vec![
                                    SSAValue {
                                        val: 4,
                                        ty: Some(TirType::F64),
                                    },
                                    SSAValue {
                                        val: 5,
                                        ty: Some(TirType::F64),
                                    },
                                ],
                            ),
                            TIR::FConst(7, 1.0, TirType::F64),
                            TIR::FConst(8, 1.0, TirType::F64),
                            TIR::CreateStructLiteral(
                                9,
                                TirType::StructInterface(vec![TirType::F64, TirType::F64]),
                                vec![
                                    SSAValue {
                                        val: 7,
                                        ty: Some(TirType::F64),
                                    },
                                    SSAValue {
                                        val: 8,
                                        ty: Some(TirType::F64),
                                    },
                                ],
                            ),
                            TIR::FConst(10, -1.0, TirType::F64),
                            TIR::FConst(11, -1.0, TirType::F64),
                            TIR::CreateStructLiteral(
                                12,
                                TirType::StructInterface(vec![TirType::F64, TirType::F64]),
                                vec![
                                    SSAValue {
                                        val: 10,
                                        ty: Some(TirType::F64),
                                    },
                                    SSAValue {
                                        val: 11,
                                        ty: Some(TirType::F64),
                                    },
                                ],
                            ),
                            TIR::IConst(13, 3, TirType::I64),
                            TIR::IConst(14, 8, TirType::I64),
                            TIR::IConst(15, 1, TirType::I64),
                            TIR::CallExternFunction(
                                16,
                                Box::new("toy_malloc_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 13,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 14,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 15,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                true,
                                TirType::Ptr,
                            ),
                            TIR::IConst(17, 0, TirType::I64),
                            TIR::IConst(18, 8, TirType::I64),
                            TIR::CallExternFunction(
                                19,
                                Box::new("toy_write_to_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 16,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 6,
                                        ty: Some(TirType::StructInterface(vec![
                                            TirType::F64,
                                            TirType::F64,
                                        ])),
                                    },
                                    SSAValue {
                                        val: 17,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 18,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::Void,
                            ),
                            TIR::IConst(20, 1, TirType::I64),
                            TIR::IConst(21, 8, TirType::I64),
                            TIR::CallExternFunction(
                                22,
                                Box::new("toy_write_to_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 16,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 9,
                                        ty: Some(TirType::StructInterface(vec![
                                            TirType::F64,
                                            TirType::F64,
                                        ])),
                                    },
                                    SSAValue {
                                        val: 20,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 21,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::Void,
                            ),
                            TIR::IConst(23, 2, TirType::I64),
                            TIR::IConst(24, 8, TirType::I64),
                            TIR::CallExternFunction(
                                25,
                                Box::new("toy_write_to_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 16,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 12,
                                        ty: Some(TirType::StructInterface(vec![
                                            TirType::F64,
                                            TirType::F64,
                                        ])),
                                    },
                                    SSAValue {
                                        val: 23,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 24,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::Void,
                            ),
                            TIR::IConst(26, 0, TirType::I64),
                            TIR::JumpBlockUnCond(27, 2),
                        ],
                    },
                    Block {
                        id: 2,
                        ins: vec![
                            TIR::Phi(
                                28,
                                vec![0, 3],
                                vec![
                                    SSAValue {
                                        val: 16,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 28,
                                        ty: Some(TirType::Ptr),
                                    },
                                ],
                            ),
                            TIR::Phi(
                                29,
                                vec![0, 3],
                                vec![
                                    SSAValue {
                                        val: 26,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 41,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                            ),
                            TIR::CallExternFunction(
                                30,
                                Box::new("toy_arrlen".to_string()),
                                vec![SSAValue {
                                    val: 28,
                                    ty: Some(TirType::Ptr),
                                }],
                                false,
                                TirType::I64,
                            ),
                            TIR::BoolInfix(
                                31,
                                SSAValue {
                                    val: 29,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 30,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::LessThan,
                            ),
                            TIR::JumpCond(
                                32,
                                SSAValue {
                                    val: 31,
                                    ty: Some(TirType::I1),
                                },
                                3,
                                4,
                            ),
                        ],
                    },
                    Block {
                        id: 3,
                        ins: vec![
                            TIR::CallExternFunction(
                                33,
                                Box::new("toy_read_from_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 28,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 29,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::I64,
                            ),
                            TIR::FConst(34, 5.0, TirType::F64),
                            TIR::IConst(35, 0, TirType::I64),
                            TIR::FConst(36, 2.0, TirType::F64),
                            TIR::ItoF(
                                37,
                                SSAValue {
                                    val: 35,
                                    ty: Some(TirType::I64),
                                },
                                TirType::F64,
                            ),
                            TIR::NumericInfix(
                                38,
                                SSAValue {
                                    val: 37,
                                    ty: Some(TirType::F64),
                                },
                                SSAValue {
                                    val: 36,
                                    ty: Some(TirType::F64),
                                },
                                NumericInfixOp::Minus,
                            ),
                            TIR::CallLocalFunction(
                                39,
                                Box::new("Point:::move_struct_float_float".to_string()),
                                vec![
                                    SSAValue {
                                        val: 33,
                                        ty: Some(TirType::StructInterface(vec![
                                            TirType::F64,
                                            TirType::F64,
                                        ])),
                                    },
                                    SSAValue {
                                        val: 34,
                                        ty: Some(TirType::F64),
                                    },
                                    SSAValue {
                                        val: 38,
                                        ty: Some(TirType::F64),
                                    },
                                ],
                                false,
                                TirType::Void,
                            ),
                            TIR::IConst(40, 1, TirType::I64),
                            TIR::NumericInfix(
                                41,
                                SSAValue {
                                    val: 29,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 40,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Plus,
                            ),
                            TIR::JumpBlockUnCond(42, 2),
                        ],
                    },
                    Block {
                        id: 4,
                        ins: vec![
                            TIR::IConst(43, 4, TirType::I64),
                            TIR::IConst(44, 1, TirType::I64),
                            TIR::CallExternFunction(
                                45,
                                Box::new("toy_println".to_string()),
                                vec![
                                    SSAValue {
                                        val: 28,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 43,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 44,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::Void,
                            ),
                            TIR::IConst(46, 0, TirType::I64),
                            TIR::Ret(
                                47,
                                SSAValue {
                                    val: 46,
                                    ty: Some(TirType::I64),
                                },
                            ),
                        ],
                    },
                ],
                ins_counter: 48,
                ret_type: TirType::I64,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            Function {
                params: vec![
                    SSAValue {
                        val: 1,
                        ty: Some(TirType::StructInterface(vec![TirType::F64, TirType::F64])),
                    },
                    SSAValue {
                        val: 2,
                        ty: Some(TirType::F64),
                    },
                    SSAValue {
                        val: 3,
                        ty: Some(TirType::F64),
                    },
                ],
                name: Box::new("Point:::move_struct_float_float".to_string()),
                body: vec![Block {
                    id: 1,
                    ins: vec![
                        TIR::ReadStructLiteral(
                            4,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::StructInterface(vec![
                                    TirType::F64,
                                    TirType::F64,
                                ])),
                            },
                            0,
                        ),
                        TIR::NumericInfix(
                            5,
                            SSAValue {
                                val: 4,
                                ty: Some(TirType::F64),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::F64),
                            },
                            NumericInfixOp::Plus,
                        ),
                        TIR::WriteStructLiteral(
                            6,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::StructInterface(vec![
                                    TirType::F64,
                                    TirType::F64,
                                ])),
                            },
                            0,
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::F64),
                            },
                        ),
                        TIR::ReadStructLiteral(
                            7,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::StructInterface(vec![
                                    TirType::F64,
                                    TirType::F64,
                                ])),
                            },
                            1,
                        ),
                        TIR::NumericInfix(
                            8,
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::F64),
                            },
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::F64),
                            },
                            NumericInfixOp::Plus,
                        ),
                        TIR::WriteStructLiteral(
                            9,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::StructInterface(vec![
                                    TirType::F64,
                                    TirType::F64,
                                ])),
                            },
                            1,
                            SSAValue {
                                val: 8,
                                ty: Some(TirType::F64),
                            },
                        ),
                        TIR::Ret(10, SSAValue { val: 0, ty: None }),
                    ],
                }],
                ins_counter: 11,
                ret_type: TirType::Void,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    );
}

#[test]
fn test_tirgen_extern_func_dec_and_call() {
    setup_tir!(
        res,
        "extern fn printf(msg: str): int;
         printf(\"Hello\")"
    );
    compare_tir(
        "extern_func_dec_and_call",
        res,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::GlobalString(0, Box::new("Hello".to_string())),
                    TIR::CallExternFunction(
                        1,
                        Box::new("toy_malloc".to_string()),
                        vec![SSAValue {
                            val: 0,
                            ty: Some(TirType::Ptr),
                        }],
                        true,
                        TirType::Ptr,
                    ),
                    TIR::CallExternFunction(
                        2,
                        Box::new("printf".to_string()),
                        vec![SSAValue {
                            val: 1,
                            ty: Some(TirType::Ptr),
                        }],
                        false,
                        TirType::I64,
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 5,
            ret_type: TirType::I64,
            heap_allocations: vec![crate::codegen::tir::ir::HeapAllocation {
                block: 0,
                allocation_id: 0,
                function: Box::new("user_main".to_string()),
                refs: vec![(Box::new("user_main".to_string()), 0, 1)],
                alloc_ins: SSAValue {
                    val: 1,
                    ty: Some(TirType::Ptr),
                },
            }],
            heap_counter: 1,
        }],
    );
}

#[test]
fn test_tirgen_import_stmt() {
    setup_tir!(
        ir,
        r#"import std.math;
        let x = math.abs(-5);"#
    );
    compare_tir(
        "import_stmt",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, -5, TirType::I64),
                    TIR::CallExternFunction(
                        1,
                        Box::new("std::math::abs_int".to_string()),
                        vec![SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        }],
                        false,
                        TirType::I64,
                    ),
                    TIR::IConst(2, 0, TirType::I64),
                    TIR::Ret(
                        3,
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 4,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    );
}

#[test]
fn test_tirgen_argv() {
    setup_tir!(ir, "import std.sys; let args = sys.argv(); println(args);");
    compare_tir(
        "argv",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::CallExternFunction(
                        0,
                        Box::new("std::sys::argv".to_string()),
                        vec![],
                        true,
                        TirType::Ptr,
                    ),
                    TIR::IConst(1, 4, TirType::I64),
                    TIR::IConst(2, 1, TirType::I64),
                    TIR::CallExternFunction(
                        3,
                        Box::new("toy_println".to_string()),
                        vec![
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::Ptr),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(4, 0, TirType::I64),
                    TIR::Ret(
                        5,
                        SSAValue {
                            val: 4,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ret_type: TirType::I64,
            ins_counter: 6,
            heap_allocations: vec![crate::codegen::tir::ir::HeapAllocation {
                block: 0,
                allocation_id: 0,
                function: Box::new("user_main".to_string()),
                refs: vec![(Box::new("user_main".to_string()), 0, 3)],
                alloc_ins: SSAValue {
                    val: 0,
                    ty: Some(TirType::Ptr),
                },
            }],
            heap_counter: 1,
        }],
    );
}

#[test]
fn test_tirgen_weird_loop_bug() {
    setup_tir!(
        ir,
        "
        export fn concat(arr: any[], elem: any): any[]{
            let newArr: any[] = [];
            let i = 0;
            while i < len(arr) {
                newArr[i] = arr[i];
                i++;
            }
            newArr[i] = elem;
            return newArr;
        }"
    );
    compare_tir(
        "weird_loop_bug",
        ir,
        vec![
            Function {
                params: vec![],
                name: Box::new("user_main".to_string()),
                ret_type: TirType::I64,
                body: vec![Block {
                    id: 0,
                    ins: vec![
                        TIR::IConst(2, 0, TirType::I64),
                        TIR::Ret(
                            3,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                }],
                ins_counter: 4,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            Function {
                params: vec![
                    SSAValue {
                        val: 0,
                        ty: Some(TirType::Ptr),
                    },
                    SSAValue {
                        val: 1,
                        ty: Some(TirType::Ptr),
                    },
                ],
                name: Box::new("concat_anyarr_any".to_string()),
                ret_type: TirType::Ptr,
                body: vec![
                    Block {
                        id: 1,
                        ins: vec![
                            TIR::IConst(2, 0, TirType::I64),
                            TIR::IConst(3, 0, TirType::I64),
                            TIR::IConst(4, 1, TirType::I64),
                            TIR::CallExternFunction(
                                5,
                                Box::new("toy_malloc_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 2,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 3,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 4,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                true,
                                TirType::Ptr,
                            ),
                            TIR::IConst(6, 0, TirType::I64),
                            TIR::JumpBlockUnCond(7, 2),
                        ],
                    },
                    Block {
                        id: 2,
                        ins: vec![
                            TIR::Phi(
                                8,
                                vec![1, 3],
                                vec![
                                    SSAValue {
                                        val: 5,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 8,
                                        ty: Some(TirType::Ptr),
                                    },
                                ],
                            ),
                            TIR::Phi(
                                9,
                                vec![1, 3],
                                vec![
                                    SSAValue {
                                        val: 6,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 19,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                            ),
                            TIR::Phi(
                                10,
                                vec![1, 3],
                                vec![
                                    SSAValue {
                                        val: 1,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 10,
                                        ty: Some(TirType::Ptr),
                                    },
                                ],
                            ),
                            TIR::Phi(
                                11,
                                vec![1, 3],
                                vec![
                                    SSAValue {
                                        val: 0,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 11,
                                        ty: Some(TirType::Ptr),
                                    },
                                ],
                            ),
                            TIR::CallExternFunction(
                                12,
                                Box::new("toy_arrlen".to_string()),
                                vec![SSAValue {
                                    val: 11,
                                    ty: Some(TirType::Ptr),
                                }],
                                false,
                                TirType::I64,
                            ),
                            TIR::BoolInfix(
                                13,
                                SSAValue {
                                    val: 9,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 12,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::LessThan,
                            ),
                            TIR::JumpCond(
                                14,
                                SSAValue {
                                    val: 13,
                                    ty: Some(TirType::I1),
                                },
                                3,
                                4,
                            ),
                        ],
                    },
                    Block {
                        id: 3,
                        ins: vec![
                            TIR::CallExternFunction(
                                15,
                                Box::new("toy_read_from_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 11,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 9,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::I64,
                            ),
                            TIR::IConst(16, 0, TirType::I64),
                            TIR::CallExternFunction(
                                17,
                                Box::new("toy_write_to_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 8,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 15,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 9,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 16,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::Void,
                            ),
                            TIR::IConst(18, 1, TirType::I64),
                            TIR::NumericInfix(
                                19,
                                SSAValue {
                                    val: 9,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 18,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Plus,
                            ),
                            TIR::JumpBlockUnCond(20, 2),
                        ],
                    },
                    Block {
                        id: 4,
                        ins: vec![
                            TIR::IConst(21, 0, TirType::I64),
                            TIR::CallExternFunction(
                                22,
                                Box::new("toy_write_to_arr".to_string()),
                                vec![
                                    SSAValue {
                                        val: 8,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 10,
                                        ty: Some(TirType::Ptr),
                                    },
                                    SSAValue {
                                        val: 9,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 21,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                                false,
                                TirType::Void,
                            ),
                            TIR::Ret(
                                23,
                                SSAValue {
                                    val: 8,
                                    ty: Some(TirType::Ptr),
                                },
                            ),
                        ],
                    },
                ],
                ins_counter: 24,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    );
}

#[test]
fn test_tirgen_nested_if() {
    setup_tir!(ir, "let x = 5; if x > 3 { if x < 10 { println(x) } }");
    compare_tir(
        "nested_if",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![
                Block {
                    id: 0,
                    ins: vec![
                        TIR::IConst(0, 5, TirType::I64),
                        TIR::IConst(1, 3, TirType::I64),
                        TIR::BoolInfix(
                            2,
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            BoolInfixOp::GreaterThan,
                        ),
                        TIR::JumpCond(
                            3,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I1),
                            },
                            1,
                            2,
                        ),
                    ],
                },
                Block {
                    id: 1,
                    ins: vec![
                        TIR::IConst(4, 10, TirType::I64),
                        TIR::BoolInfix(
                            5,
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 4,
                                ty: Some(TirType::I64),
                            },
                            BoolInfixOp::LessThan,
                        ),
                        TIR::JumpCond(
                            6,
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I1),
                            },
                            3,
                            4,
                        ),
                    ],
                },
                Block {
                    id: 2,
                    ins: vec![
                        TIR::IConst(12, 0, TirType::I64),
                        TIR::Ret(
                            13,
                            SSAValue {
                                val: 12,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                },
                Block {
                    id: 3,
                    ins: vec![
                        TIR::IConst(7, 2, TirType::I64),
                        TIR::IConst(8, 0, TirType::I64),
                        TIR::CallExternFunction(
                            9,
                            Box::new("toy_println".to_string()),
                            vec![
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 8,
                                    ty: Some(TirType::I64),
                                },
                            ],
                            false,
                            TirType::Void,
                        ),
                        TIR::JumpBlockUnCond(10, 4),
                    ],
                },
                Block {
                    id: 4,
                    ins: vec![TIR::JumpBlockUnCond(11, 2)],
                },
            ],
            ins_counter: 14,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    );
}

#[test]
fn test_tirgen_multiple_vars() {
    setup_tir!(ir, "let a = 1; let b = 2; let c = 3; println(a + b + c);");
    compare_tir(
        "multiple_vars",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I64),
                    TIR::IConst(1, 2, TirType::I64),
                    TIR::IConst(2, 3, TirType::I64),
                    TIR::NumericInfix(
                        3,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    TIR::NumericInfix(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    TIR::IConst(5, 2, TirType::I64),
                    TIR::IConst(6, 0, TirType::I64),
                    TIR::CallExternFunction(
                        7,
                        Box::new("toy_println".to_string()),
                        vec![
                            SSAValue {
                                val: 4,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(8, 0, TirType::I64),
                    TIR::Ret(
                        9,
                        SSAValue {
                            val: 8,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 10,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    );
}

#[test]
fn test_tirgen_division_and_modulo() {
    setup_tir!(ir, "let x = 10 / 3; let y = 10 % 3;");
    compare_tir(
        "division_and_modulo",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 10, TirType::I64),
                    TIR::IConst(1, 3, TirType::I64),
                    TIR::NumericInfix(
                        2,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Divide,
                    ),
                    TIR::IConst(3, 10, TirType::I64),
                    TIR::IConst(4, 3, TirType::I64),
                    TIR::NumericInfix(
                        5,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 4,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Modulo,
                    ),
                    TIR::IConst(6, 0, TirType::I64),
                    TIR::Ret(
                        7,
                        SSAValue {
                            val: 6,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 8,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    );
}

#[test]
fn test_tirgen_comparison_ops() {
    setup_tir!(ir, "let a = 5 >= 3; let b = 2 <= 4; let c = 1 != 2;");
    compare_tir(
        "comparison_ops",
        ir,
        vec![Function {
            params: vec![],
            name: Box::new("user_main".to_string()),
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 5, TirType::I64),
                    TIR::IConst(1, 3, TirType::I64),
                    TIR::BoolInfix(
                        2,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64),
                        },
                        BoolInfixOp::GreaterThanEqt,
                    ),
                    TIR::IConst(3, 2, TirType::I64),
                    TIR::IConst(4, 4, TirType::I64),
                    TIR::BoolInfix(
                        5,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 4,
                            ty: Some(TirType::I64),
                        },
                        BoolInfixOp::LessThenEqt,
                    ),
                    TIR::IConst(6, 1, TirType::I64),
                    TIR::IConst(7, 2, TirType::I64),
                    TIR::BoolInfix(
                        8,
                        SSAValue {
                            val: 6,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 7,
                            ty: Some(TirType::I64),
                        },
                        BoolInfixOp::NotEquals,
                    ),
                    TIR::IConst(9, 0, TirType::I64),
                    TIR::Ret(
                        10,
                        SSAValue {
                            val: 9,
                            ty: Some(TirType::I64),
                        },
                    ),
                ],
            }],
            ins_counter: 11,
            ret_type: TirType::I64,
            heap_allocations: vec![],
            heap_counter: 0,
        }],
    );
}

#[test]
fn test_tirgen_break_continue_lowering() {
    setup_tir!(
        ir,
        "fn loop(): int { let x = 0; while x < 10 { x++; if x == 3 { continue; } if x == 7 { break; } } return x; } loop();"
    );
    compare_tir(
        "break_continue_lowering",
        ir,
        vec![
            Function {
                params: vec![],
                body: vec![Block {
                    id: 0,
                    ins: vec![
                        TIR::CallLocalFunction(
                            0,
                            Box::new("loop".to_string()),
                            vec![],
                            false,
                            TirType::I64,
                        ),
                        TIR::IConst(1, 0, TirType::I64),
                        TIR::Ret(
                            2,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                        ),
                    ],
                }],
                name: Box::new("user_main".to_string()),
                ins_counter: 3,
                ret_type: TirType::I64,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            Function {
                params: vec![],
                body: vec![
                    Block {
                        id: 1,
                        ins: vec![TIR::IConst(0, 0, TirType::I64), TIR::JumpBlockUnCond(1, 2)],
                    },
                    Block {
                        id: 2,
                        ins: vec![
                            TIR::Phi(
                                2,
                                vec![1, 5],
                                vec![
                                    SSAValue {
                                        val: 0,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 17,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                            ),
                            TIR::IConst(3, 10, TirType::I64),
                            TIR::BoolInfix(
                                4,
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 3,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::LessThan,
                            ),
                            TIR::JumpCond(
                                5,
                                SSAValue {
                                    val: 4,
                                    ty: Some(TirType::I1),
                                },
                                3,
                                4,
                            ),
                        ],
                    },
                    Block {
                        id: 3,
                        ins: vec![
                            TIR::IConst(6, 1, TirType::I64),
                            TIR::NumericInfix(
                                7,
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 6,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Plus,
                            ),
                            TIR::IConst(8, 3, TirType::I64),
                            TIR::BoolInfix(
                                9,
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 8,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::Equals,
                            ),
                            TIR::JumpCond(
                                10,
                                SSAValue {
                                    val: 9,
                                    ty: Some(TirType::I1),
                                },
                                6,
                                7,
                            ),
                        ],
                    },
                    Block {
                        id: 4,
                        ins: vec![TIR::Ret(
                            19,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                        )],
                    },
                    Block {
                        id: 5,
                        ins: vec![
                            TIR::Phi(
                                17,
                                vec![6, 9],
                                vec![
                                    SSAValue {
                                        val: 7,
                                        ty: Some(TirType::I64),
                                    },
                                    SSAValue {
                                        val: 7,
                                        ty: Some(TirType::I64),
                                    },
                                ],
                            ),
                            TIR::JumpBlockUnCond(18, 2),
                        ],
                    },
                    Block {
                        id: 6,
                        ins: vec![TIR::JumpBlockUnCond(11, 5)],
                    },
                    Block {
                        id: 7,
                        ins: vec![
                            TIR::IConst(12, 7, TirType::I64),
                            TIR::BoolInfix(
                                13,
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 12,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::Equals,
                            ),
                            TIR::JumpCond(
                                14,
                                SSAValue {
                                    val: 13,
                                    ty: Some(TirType::I1),
                                },
                                8,
                                9,
                            ),
                        ],
                    },
                    Block {
                        id: 8,
                        ins: vec![TIR::JumpBlockUnCond(15, 4)],
                    },
                    Block {
                        id: 9,
                        ins: vec![TIR::JumpBlockUnCond(16, 5)],
                    },
                ],
                name: Box::new("loop".to_string()),
                ins_counter: 20,
                ret_type: TirType::I64,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    );
}
