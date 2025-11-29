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
                ins: vec![TIR::IConst(0, 5, TirType::I64)]
            }],

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
                ins: vec![TIR::IConst(0, 1, TirType::I1)]
            }],
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
                ]
            }],
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
                ]
            }],
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
                    )
                ]
            }],
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
                    )
                ]
            }],
            ret_type: TirType::I64
        }]

    )
}

#[test]
fn test_tirgen_if_stmt() {
    
}