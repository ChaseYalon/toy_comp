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
#[test]
fn test_tirgen_int_lit() {
    setup_tir!(ir, "5");
    assert_eq!(
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
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 3,
            ret_type: TirType::I64
        }]
    )
}
#[test]
fn test_tirgen_bool_lit() {
    setup_tir!(ir, "true");
    assert_eq!(
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
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 3,
            ret_type: TirType::I64
        }]
    )
}
#[test]
fn test_tirgen_numeric_infix() {
    setup_tir!(ir, "5 + 3 * 9");
    assert_eq!(
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
                            ty: Some(TirType::I64)
                        },
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64)
                        },
                        NumericInfixOp::Multiply
                    ),
                    TIR::NumericInfix(
                        4,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64)
                        },
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64)
                        },
                        NumericInfixOp::Plus
                    ),
                    TIR::IConst(5, 0, TirType::I64),
                    TIR::Ret(
                        6,
                        SSAValue {
                            val: 5,
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 7,
            ret_type: TirType::I64
        }]
    )
}
#[test]
fn test_tirgen_boolean_infix() {
    setup_tir!(ir, "true && false");
    assert_eq!(
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
                            ty: Some(TirType::I1)
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I1)
                        },
                        BoolInfixOp::And
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 5, 
            ret_type: TirType::I64
        }]
    )
}

#[test]
fn test_tirgen_var_dec_and_reassign() {
    setup_tir!(ir, "let x = 9; x += 3");
    assert_eq!(
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
                            ty: Some(TirType::I64)
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64)
                        },
                        NumericInfixOp::Plus
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 5,
            ret_type: TirType::I64
        }]
    )
}

#[test]
fn test_tirgen_var_ref() {
    setup_tir!(ir, "let x = 9; x + 4");
    assert_eq!(
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
                            ty: Some(TirType::I64)
                        },
                        SSAValue {
                            val: 1,
                            ty: Some(TirType::I64)
                        },
                        NumericInfixOp::Plus
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 5,
            ret_type: TirType::I64
        }]
    )
}

#[test]
fn test_tirgen_if_stmt() {
    setup_tir!(ir, "let x = true || false; if x {5}; 9 + 3;");
    assert_eq!(
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
                                ty: Some(TirType::I1)
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I1)
                            },
                            BoolInfixOp::Or
                        ),
                        TIR::JumpCond(
                            3,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I1)
                            },
                            1,
                            2
                        )
                    ]
                },
                Block {
                    id: 1,
                    ins: vec![TIR::IConst(4, 5, TirType::I64), TIR::JumpBlockUnCond(5, 2)]
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
                                ty: Some(TirType::I64)
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64)
                            },
                            NumericInfixOp::Plus
                        ),
                        TIR::IConst(9, 0, TirType::I64),
                        TIR::Ret(
                            10,
                            SSAValue {
                                val: 9,
                                ty: Some(TirType::I64)
                            }
                        )
                    ]
                }
            ],
            ins_counter: 11,
            ret_type: TirType::I64
        }]
    )
}

#[test]
fn test_tirgen_if_else_stmt() {
    setup_tir!(ir, "let x = true || false; if x {5} else {9 + 3};");
    assert_eq!(
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
                                ty: Some(TirType::I1)
                            },
                            SSAValue {
                                val: 1,
                                ty: Some(TirType::I1)
                            },
                            BoolInfixOp::Or
                        ),
                        TIR::JumpCond(
                            3,
                            SSAValue {
                                val: 2,
                                ty: Some(TirType::I1)
                            },
                            1,
                            2
                        )
                    ]
                },
                Block {
                    id: 1,
                    ins: vec![TIR::IConst(4, 5, TirType::I64), TIR::JumpBlockUnCond(5, 3)]
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
                                ty: Some(TirType::I64)
                            },
                            SSAValue {
                                val: 7,
                                ty: Some(TirType::I64)
                            },
                            NumericInfixOp::Plus
                        ),
                        TIR::JumpBlockUnCond(9, 3)
                    ]
                },
                Block {
                    id: 3,
                    ins: vec![
                        TIR::IConst(10, 0, TirType::I64),
                        TIR::Ret(
                            11,
                            SSAValue {
                                val: 10,
                                ty: Some(TirType::I64)
                            }
                        )
                    ]
                }
            ],
            ins_counter: 12,
            ret_type: TirType::I64
        }]
    )
}


#[test]
fn test_tirgen_empty_expr() {
    setup_tir!(ir, "let x = 9 * (4 + 3)");
    assert_eq!(
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
                            ty: Some(TirType::I64)
                        },
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64)
                        },
                        NumericInfixOp::Plus
                    ),
                    TIR::NumericInfix(
                        4,
                        SSAValue {
                            val: 0,
                            ty: Some(TirType::I64)
                        },
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64)
                        },
                        NumericInfixOp::Multiply
                    ),
                    TIR::IConst(5, 0, TirType::I64),
                    TIR::Ret(
                        6,
                        SSAValue {
                            val: 5,
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 7,
            ret_type: TirType::I64
        }]
    )
}

#[test]
fn test_tirgen_func_call() {
    setup_tir!(ir, "fn add(a: int, b: int): int { return a + b }; add(3, 5)");
    assert_eq!(
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
                                SSAValue { val: 2, ty: Some(TirType::I64) },
                                SSAValue { val: 3, ty: Some(TirType::I64) }
                            ],
                            false
                        ),
                        TIR::IConst(5, 0, TirType::I64),
                        TIR::Ret(
                            6,
                            SSAValue {
                                val: 5,
                                ty: Some(TirType::I64)
                            }
                        )
                    ]
                }],
                ins_counter: 7,
                ret_type: TirType::I64
            },
            // add functionK
            Function {
                params: vec![
                    SSAValue { val: 0, ty: Some(TirType::I64) },
                    SSAValue { val: 1, ty: Some(TirType::I64) }
                ],
                name: Box::new("add".to_string()),
                body: vec![Block {
                    id: 1,
                    ins: vec![
                        TIR::NumericInfix(
                            0,
                            SSAValue { val: 0, ty: Some(TirType::I64) },
                            SSAValue { val: 1, ty: Some(TirType::I64) },
                            NumericInfixOp::Plus
                        ),
                        TIR::Ret(
                            1,
                            SSAValue {
                                val: 0,
                                ty: Some(TirType::I64)
                            }
                        )
                    ]
                }],
                ins_counter: 2,
                ret_type: TirType::I64
            }
        ]
    )
}

#[test]
fn test_tirgen_while_stmt_break_continue() {
    setup_tir!(ir, "let x = 0; while x < 10 {if x == 4 {continue} if x == 9{break} x++}");
    assert_eq!(
        ir,
        vec![
            Function{
                params: vec![],
                body: vec![
                    Block{
                        id: 0,
                        ins: vec![
                            TIR::IConst(0, 0, TirType::I64),
                            
                        ]
                    }
                ],
                name: Box::new("user_main".to_string()),
                ret_type: TirType::I64,
                ins_counter: 0
            }
        ]
    )
}