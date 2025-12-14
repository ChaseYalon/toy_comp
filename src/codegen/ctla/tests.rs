/*
use crate::Lexer;
use crate::Parser;
use crate::codegen::Block;
use crate::codegen::Function;
use crate::codegen::Generator;
use crate::codegen::SSAValue;
use crate::codegen::TIR;
use crate::codegen::TirType;
use crate::codegen::tir::ir::NumericInfixOp;
macro_rules! setup_tir {
    ($o: ident, $v:expr) => {
        let mut l = Lexer::new();
        let mut p = Parser::new();
        let mut t = Generator::new();
        let toks = l.lex($v.to_string()).unwrap();
        let ast = p.parse(toks).unwrap(); //I dont like .unwrap(0)
        let $o = t.generate(ast).unwrap();
    };
}
#[test]
fn test_ctla_str() {
    setup_tir!(ir, r#"let x = "hello""#);
    assert_eq!(
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            ret_type: TirType::I64,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::GlobalString(0, Box::new("hello".to_string())),
                    TIR::CallExternFunction(
                        1,
                        Box::new("toy_malloc".to_string()),
                        vec![SSAValue {
                            val: 0,
                            ty: Some(TirType::I8PTR)
                        }],
                        true,
                        TirType::I8PTR
                    ),
                    TIR::CallExternFunction(
                        2,
                        Box::new("toy_free".to_string()),
                        vec![SSAValue {
                            val: 1,
                            ty: Some(TirType::I8PTR)
                        }],
                        false,
                        TirType::Void
                    ),
                    TIR::IConst(3, 0, TirType::I64),
                    TIR::Ret(
                        4,
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 4,
            heap_allocations: vec![],
            heap_counter: 0
        }]
    )
}

#[test]
fn test_ctla_str_inbetween() {
    setup_tir!(ir, r#"let x = "hi"; 9 + 3; println(x);"#);
    assert_eq!(
        ir,
        vec![Function {
            name: Box::new("user_main".to_string()),
            params: vec![],
            ret_type: TirType::I64,
            body: vec![Block {
                id: 0,
                ins: vec![
                    TIR::GlobalString(0, Box::new("hi".to_string())),
                    TIR::CallExternFunction(
                        1,
                        Box::new("toy_malloc".to_string()),
                        vec![SSAValue {
                            val: 0,
                            ty: Some(TirType::I8PTR)
                        }],
                        true,
                        TirType::I8PTR
                    ),
                    TIR::IConst(2, 9, TirType::I64),
                    TIR::IConst(3, 3, TirType::I64),
                    TIR::NumericInfix(
                        4,
                        SSAValue {
                            val: 2,
                            ty: Some(TirType::I64)
                        },
                        SSAValue {
                            val: 3,
                            ty: Some(TirType::I64)
                        },
                        NumericInfixOp::Plus
                    ),
                    TIR::CallExternFunction(
                        5,
                        Box::new("toy_println".to_string()),
                        vec![SSAValue {
                            val: 1,
                            ty: Some(TirType::I8PTR)
                        }],
                        false,
                        TirType::Void
                    ),
                    TIR::CallExternFunction(
                        6,
                        Box::new("toy_free".to_string()),
                        vec![SSAValue {
                            val: 1,
                            ty: Some(TirType::I8PTR)
                        }],
                        false,
                        TirType::Void
                    ),
                    TIR::IConst(7, 0, TirType::I64),
                    TIR::Ret(
                        8,
                        SSAValue {
                            val: 7,
                            ty: Some(TirType::I64)
                        }
                    )
                ]
            }],
            ins_counter: 9,
            heap_allocations: vec![],
            heap_counter: 0
        }]
    )
}

*/
