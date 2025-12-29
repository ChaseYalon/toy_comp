use crate::codegen::AstToIrConverter;
use crate::codegen::tir::ir::{
    Block, BoolInfixOp, Function, NumericInfixOp, SSAValue, TIR, TirType,
};
use crate::lexer::Lexer;
use crate::parser::Parser;
macro_rules! setup_tir {
    ($o: ident, $v:expr) => {
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut t = AstToIrConverter::new();
        let toks = l.lex($v.to_string()).unwrap();
        let ast = p.parse(toks).unwrap(); //I dont like .unwrap(0)
        let $o = t.convert(ast).unwrap();
    };
}
fn compare_tir(a: Vec<Function>, b: Vec<Function>) {
    if a.len() != b.len() {
        panic!(
            "[ERROR] Generated: {} functions, got {} functions",
            a.len(),
            b.len()
        );
    }
    for (i, func) in a.iter().enumerate() {
        //g_ = generated
        //r_ = received
        let Function {
            body: g_body,
            name: g_name,
            ret_type: g_ret_type,
            params: g_params,
            ..
        } = func.clone();
        let Function {
            body: r_body,
            name: r_name,
            ret_type: r_ret_type,
            params: r_params,
            ..
        } = b[i].clone();
        assert_eq!(g_body, r_body);
        assert_eq!(*g_name, *r_name);
        assert_eq!(g_ret_type, r_ret_type);
    }
}
#[test]
fn test_tirgen_int_lit() {
    setup_tir!(ir, "5");
    compare_tir(
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
                            Box::new("add".to_string()),
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
                name: Box::new("add".to_string()),
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
                            ty: Some(TirType::I8PTR),
                        }],
                        true,
                        TirType::I64,
                    ),
                    TIR::GlobalString(2, Box::new("fee".to_string())),
                    TIR::CallExternFunction(
                        3,
                        Box::new("toy_malloc".to_string()),
                        vec![SSAValue {
                            val: 2,
                            ty: Some(TirType::I8PTR),
                        }],
                        true,
                        TirType::I64,
                    ),
                    TIR::CallExternFunction(
                        4,
                        Box::new("toy_concat".to_string()),
                        vec![
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I8PTR),
                            },
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::I8PTR),
                            },
                        ],
                        true,
                        TirType::I64,
                    ),
                    TIR::CallExternFunction(
                        5,
                        Box::new("toy_strequal".to_string()),
                        vec![
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I8PTR),
                            },
                            SSAValue {
                                val: 3,
                                ty: Some(TirType::I8PTR),
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
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            ret_type: TirType::I64,
            ins_counter: 25,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I64),
                    TIR::IConst(1, 2, TirType::I64),
                    TIR::IConst(2, 3, TirType::I64),
                    TIR::IConst(3, 3, TirType::I64),
                    TIR::IConst(4, 6, TirType::I64),
                    TIR::CallExternFunction(
                        5,
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
                        ],
                        true,
                        TirType::I64,
                    ),
                    TIR::IConst(6, 0, TirType::I64),
                    TIR::IConst(7, 6, TirType::I64),
                    TIR::CallExternFunction(
                        8,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(9, 1, TirType::I64),
                    TIR::IConst(10, 6, TirType::I64),
                    TIR::CallExternFunction(
                        11,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 9,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 10,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(12, 2, TirType::I64),
                    TIR::IConst(13, 6, TirType::I64),
                    TIR::CallExternFunction(
                        14,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 12,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 13,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(15, 2, TirType::I64),
                    TIR::IConst(16, 9, TirType::I64),
                    TIR::IConst(17, 6, TirType::I64),
                    TIR::CallExternFunction(
                        18,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 16,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 15,
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
                    TIR::IConst(19, 1, TirType::I64),
                    TIR::CallExternFunction(
                        20,
                        Box::new("toy_read_from_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 19,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::I64,
                    ),
                    TIR::IConst(21, 3, TirType::I64),
                    TIR::NumericInfix(
                        22,
                        SSAValue {
                            val: 20,
                            ty: Some(TirType::I64),
                        },
                        SSAValue {
                            val: 21,
                            ty: Some(TirType::I64),
                        },
                        NumericInfixOp::Plus,
                    ),
                    // Implicit return 0
                    TIR::IConst(23, 0, TirType::I64),
                    TIR::Ret(
                        24,
                        SSAValue {
                            val: 23,
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
                            Box::new("fib".to_string()),
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
                ins_counter: 6,
                heap_allocations: vec![],
                heap_counter: 0,
            },
            Function {
                name: Box::new("fib".to_string()),
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
                            TIR::JumpBlockUnCond(6, 3),
                        ],
                    },
                    Block {
                        id: 3,
                        ins: vec![
                            TIR::IConst(7, 1, TirType::I64),
                            TIR::BoolInfix(
                                8,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::I64),
                                },
                                BoolInfixOp::Equals,
                            ),
                            TIR::JumpCond(
                                9,
                                SSAValue {
                                    val: 8,
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
                            TIR::IConst(10, 1, TirType::I64),
                            TIR::Ret(
                                11,
                                SSAValue {
                                    val: 10,
                                    ty: Some(TirType::I64),
                                },
                            ),
                            TIR::JumpBlockUnCond(12, 5),
                        ],
                    },
                    Block {
                        id: 5,
                        ins: vec![
                            TIR::IConst(13, 1, TirType::I64),
                            TIR::NumericInfix(
                                14,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 13,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Minus,
                            ),
                            TIR::CallLocalFunction(
                                15,
                                Box::new("fib".to_string()),
                                vec![SSAValue {
                                    val: 14,
                                    ty: Some(TirType::I64),
                                }],
                                false,
                                TirType::I64,
                            ),
                            TIR::IConst(16, 2, TirType::I64),
                            TIR::NumericInfix(
                                17,
                                SSAValue {
                                    val: 0,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 16,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Minus,
                            ),
                            TIR::CallLocalFunction(
                                18,
                                Box::new("fib".to_string()),
                                vec![SSAValue {
                                    val: 17,
                                    ty: Some(TirType::I64),
                                }],
                                false,
                                TirType::I64,
                            ),
                            TIR::NumericInfix(
                                19,
                                SSAValue {
                                    val: 15,
                                    ty: Some(TirType::I64),
                                },
                                SSAValue {
                                    val: 18,
                                    ty: Some(TirType::I64),
                                },
                                NumericInfixOp::Plus,
                            ),
                            TIR::Ret(
                                20,
                                SSAValue {
                                    val: 19,
                                    ty: Some(TirType::I64),
                                },
                            ),
                        ],
                    },
                ],
                ins_counter: 21,
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
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            ret_type: TirType::I64,
            ins_counter: 25,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::IConst(0, 1, TirType::I64),
                    TIR::IConst(1, 2, TirType::I64),
                    TIR::IConst(2, 3, TirType::I64),
                    TIR::IConst(3, 3, TirType::I64),
                    TIR::IConst(4, 6, TirType::I64),
                    TIR::CallExternFunction(
                        5,
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
                        ],
                        true,
                        TirType::I64,
                    ),
                    TIR::IConst(6, 0, TirType::I64),
                    TIR::IConst(7, 6, TirType::I64),
                    TIR::CallExternFunction(
                        8,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 6,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(9, 1, TirType::I64),
                    TIR::IConst(10, 6, TirType::I64),
                    TIR::CallExternFunction(
                        11,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 9,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 10,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(12, 2, TirType::I64),
                    TIR::IConst(13, 6, TirType::I64),
                    TIR::CallExternFunction(
                        14,
                        Box::new("toy_write_to_arr".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 12,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 13,
                                ty: Some(TirType::I64),
                            },
                        ],
                        false,
                        TirType::Void,
                    ),
                    TIR::IConst(15, 6, TirType::I64),
                    TIR::IConst(16, 1, TirType::I64),
                    TIR::CallExternFunction(
                        17,
                        Box::new("toy_println".to_string()),
                        vec![
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64),
                            },
                            SSAValue {
                                val: 15,
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
                    TIR::IConst(18, 0, TirType::I64),
                    TIR::Ret(
                        19,
                        SSAValue {
                            val: 18,
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
                            Box::new("isEven".to_string()),
                            vec![SSAValue {
                                val: 1,
                                ty: Some(TirType::I64),
                            }],
                            false,
                            TirType::I8PTR,
                        ),
                        TIR::IConst(3, 2, TirType::I64),
                        TIR::IConst(4, 0, TirType::I64),
                        TIR::CallExternFunction(
                            5,
                            Box::new("toy_println".to_string()),
                            vec![
                                SSAValue {
                                    val: 2,
                                    ty: Some(TirType::I8PTR),
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
                name: Box::new("isEven".to_string()),
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
                                    ty: Some(TirType::I8PTR),
                                }],
                                true,
                                TirType::I64,
                            ),
                            TIR::Ret(
                                8,
                                SSAValue {
                                    val: 7,
                                    ty: Some(TirType::I8PTR),
                                },
                            ),
                            TIR::JumpBlockUnCond(9, 3),
                        ],
                    },
                    Block {
                        id: 3,
                        ins: vec![
                            TIR::GlobalString(10, Box::new("it is not".to_string())),
                            TIR::CallExternFunction(
                                11,
                                Box::new("toy_malloc".to_string()),
                                vec![SSAValue {
                                    val: 10,
                                    ty: Some(TirType::I8PTR),
                                }],
                                true,
                                TirType::I64,
                            ),
                            TIR::Ret(
                                12,
                                SSAValue {
                                    val: 11,
                                    ty: Some(TirType::I8PTR),
                                },
                            ),
                        ],
                    },
                ],
                ret_type: TirType::I8PTR,
                ins_counter: 13,
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
                            Box::new("Point:::print_point".to_string()),
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
                name: Box::new("Point:::print_point".to_string()),
                body: vec![Block {
                    id: 1,
                    ins: vec![
                        TIR::ReadStructLiteral(
                            1,
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::StructInterface(vec![
                                    TirType::I64,
                                    TirType::I64,
                                ])),
                            },
                            0,
                        ),
                        TIR::IConst(2, 2, TirType::I64),
                        TIR::IConst(3, 0, TirType::I64),
                        TIR::CallExternFunction(
                            4,
                            Box::new("toy_println".to_string()),
                            vec![
                                SSAValue {
                                    val: 1,
                                    ty: Some(TirType::I64),
                                },
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
                            TirType::Void,
                        ),
                        TIR::Ret(5, SSAValue { val: 0, ty: None }),
                    ],
                }],
                ins_counter: 6,
                ret_type: TirType::Void,
                heap_allocations: vec![],
                heap_counter: 0,
            },
        ],
    );
}
