use crate::parser::ast::InfixOp;
use crate::{
    lexer::Lexer,
    parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer},
    token::TypeTok,
};
use ordered_float::OrderedFloat;
macro_rules! setup_ast {
    ($i: expr, $ast: ident) => {
        let input = $i.to_string();
        let mut l = Lexer::new();
        let mut b = Boxer::new();
        let mut a = AstGenerator::new();

        let toks = l.lex(input);
        let boxes = b.box_toks(toks);
        let $ast = a.generate(boxes);
    };
}

#[test]
fn test_ast_gen_int_literal() {
    setup_ast!("64", ast);
    assert_eq!(ast, vec![Ast::IntLit(64)])
}

#[test]
fn test_ast_gen_infix_exprs() {
    setup_ast!("18 - 3", ast);
    assert_eq!(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(18)),
            Box::new(Ast::IntLit(3)),
            InfixOp::Minus
        )]
    )
}

#[test]
fn test_ast_gen_order_ops() {
    setup_ast!("18 - 3 * 5", ast);
    assert_eq!(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(18)),
            Box::new(Ast::InfixExpr(
                Box::new(Ast::IntLit(3)),
                Box::new(Ast::IntLit(5)),
                InfixOp::Multiply
            )),
            InfixOp::Minus
        )]
    )
}

#[test]
fn test_ast_gen_var_dec() {
    setup_ast!("let x = 9;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::IntLit(9)),
        )]
    )
}

#[test]
fn test_ast_gen_var_reassign() {
    setup_ast!("let x = 9; x = 5;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9)),
            ),
            Ast::VarReassign(Box::new("x".to_string()), Box::new(Ast::IntLit(5),))
        ]
    )
}

#[test]
fn test_ast_gen_static_type() {
    setup_ast!("let x: int = 9;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::IntLit(9))
        )]
    )
}
#[test]
fn test_ast_gen_bool_lit() {
    setup_ast!("let x: bool = false;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Bool,
            Box::new(Ast::BoolLit(false))
        )]
    )
}

#[test]
fn test_ast_gen_bool_infix() {
    setup_ast!(
        "let foo = 8 > 4 || false; let x = 9; let bar = 9 == x;",
        ast
    );

    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("foo".to_string()),
                TypeTok::Bool,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::InfixExpr(
                        Box::new(Ast::IntLit(8)),
                        Box::new(Ast::IntLit(4)),
                        InfixOp::GreaterThan,
                    )),
                    Box::new(Ast::BoolLit(false)),
                    InfixOp::Or,
                ))
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9))
            ),
            Ast::VarDec(
                Box::new("bar".to_string()),
                TypeTok::Bool,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(9)),
                    Box::new(Ast::VarRef(Box::new("x".to_string()))),
                    InfixOp::Equals
                ))
            )
        ]
    )
}

#[test]
fn test_ast_gen_mixed_bool_int() {
    setup_ast!("let x = 4 + 3 < 6;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Bool,
            Box::new(Ast::InfixExpr(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(4)),
                    Box::new(Ast::IntLit(3)),
                    InfixOp::Plus
                )),
                Box::new(Ast::IntLit(6)),
                InfixOp::LessThan
            ))
        )]
    )
}
#[test]
fn test_asg_gen_modulo() {
    setup_ast!("5 % 3;", ast);
    assert_eq!(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(5)),
            Box::new(Ast::IntLit(3)),
            InfixOp::Modulo
        )]
    )
}

#[test]
fn test_ast_gen_return_bool() {
    setup_ast!("let x: bool = true; x || false;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::BoolLit(true)),
            ),
            Ast::InfixExpr(
                Box::new(Ast::VarRef(Box::new("x".to_string()))),
                Box::new(Ast::BoolLit(false)),
                InfixOp::Or
            )
        ]
    )
}

#[test]
fn test_ast_gen_if_stmt() {
    setup_ast!("let x = false; if x || true {x = true;}", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::BoolLit(false))
            ),
            Ast::IfStmt(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("x".to_string()))),
                    Box::new(Ast::BoolLit(true)),
                    InfixOp::Or,
                )),
                vec![Ast::VarReassign(
                    Box::new("x".to_string()),
                    Box::new(Ast::BoolLit(true))
                )],
                None,
            )
        ]
    )
}

#[test]
fn test_ast_gen_if_stmt_complex() {
    setup_ast!("let x:int = 8; if true {x = 4}; x;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(8)),
            ),
            Ast::IfStmt(
                Box::new(Ast::BoolLit(true)),
                vec![Ast::VarReassign(
                    Box::new("x".to_string()),
                    Box::new(Ast::IntLit(4))
                )],
                None
            ),
            Ast::VarRef(Box::new("x".to_string())),
        ]
    )
}

#[test]
fn test_ast_gen_if_else() {
    setup_ast!("if true && false {let x = 7;} else {let x = 8;}", ast);
    assert_eq!(
        ast,
        vec![Ast::IfStmt(
            Box::new(Ast::InfixExpr(
                Box::new(Ast::BoolLit(true)),
                Box::new(Ast::BoolLit(false)),
                InfixOp::And,
            )),
            vec![Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(7))
            )],
            Some(vec![Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(8))
            )])
        )]
    )
}

#[test]
fn test_ast_gen_nested_parens() {
    setup_ast!("let x = (5 * (3 + 4)) / 7;", ast);
    //This nesting is a crime against humanity
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::InfixExpr(
                Box::new(Ast::EmptyExpr(Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(5)),
                    Box::new(Ast::EmptyExpr(Box::new(Ast::InfixExpr(
                        Box::new(Ast::IntLit(3)),
                        Box::new(Ast::IntLit(4)),
                        InfixOp::Plus
                    )))),
                    InfixOp::Multiply
                )))),
                Box::new(Ast::IntLit(7)),
                InfixOp::Divide
            ))
        )]
    )
}

#[test]
fn test_ast_gen_consecutive_parens() {
    setup_ast!("let x = (5 + 2) + (3 + 1);", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::InfixExpr(
                Box::new(Ast::EmptyExpr(Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(5)),
                    Box::new(Ast::IntLit(2)),
                    InfixOp::Plus,
                )))),
                Box::new(Ast::EmptyExpr(Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(3)),
                    Box::new(Ast::IntLit(1)),
                    InfixOp::Plus
                )))),
                InfixOp::Plus
            ))
        )]
    )
}

#[test]
fn test_ast_gen_func_dec_call() {
    setup_ast!(
        "fn add(a: int, b: int): int{return a + b;} let x = add(2, 3);",
        ast
    );

    assert_eq!(
        ast,
        vec![
            Ast::FuncDec(
                Box::new("add".to_string()),
                vec![
                    Ast::FuncParam(Box::new("a".to_string()), TypeTok::Int),
                    Ast::FuncParam(Box::new("b".to_string()), TypeTok::Int)
                ],
                TypeTok::Int,
                vec![Ast::Return(Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("a".to_string()))),
                    Box::new(Ast::VarRef(Box::new("b".to_string()))),
                    InfixOp::Plus
                )))]
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::FuncCall(
                    Box::new("add".to_string()),
                    vec![Ast::IntLit(2), Ast::IntLit(3),]
                ))
            )
        ]
    )
}

#[test]
fn test_ast_gen_str_lit_and_concatenation() {
    setup_ast!(
        r#"let x: str = "hello "; let y = "world"; let z = x + y;"#,
        ast
    );
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Str,
                Box::new(Ast::StringLit(Box::new("hello ".to_string())))
            ),
            Ast::VarDec(
                Box::new("y".to_string()),
                TypeTok::Str,
                Box::new(Ast::StringLit(Box::new("world".to_string())))
            ),
            Ast::VarDec(
                Box::new("z".to_string()),
                TypeTok::Str,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("x".to_string()))),
                    Box::new(Ast::VarRef(Box::new("y".to_string()))),
                    InfixOp::Plus
                ))
            )
        ]
    )
}

#[test]
fn test_ast_gen_print_int_bool() {
    setup_ast!("println(true); println(1);", ast);

    assert_eq!(
        ast,
        vec![
            Ast::FuncCall(Box::new("println".to_string()), vec![Ast::BoolLit(true)]),
            Ast::FuncCall(Box::new("println".to_string()), vec![Ast::IntLit(1),])
        ]
    )
}

#[test]
fn test_ast_gen_fibonacci() {
    setup_ast!(
        r#"
        fn fib(n: int): int{
            if n == 0 {
                return 0;
            }
            if n == 1 {
                return 1;
            }
            return fib(n - 1) + fib(n - 2);
        }
        println(fib(5));
    "#,
        ast
    );
    assert_eq!(
        ast,
        vec![
            Ast::FuncDec(
                Box::new("fib".to_string()),
                vec![Ast::FuncParam(Box::new("n".to_string()), TypeTok::Int)],
                TypeTok::Int,
                vec![
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("n".to_string()))),
                            Box::new(Ast::IntLit(0)),
                            InfixOp::Equals
                        )),
                        vec![Ast::Return(Box::new(Ast::IntLit(0)))],
                        None
                    ),
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("n".to_string()))),
                            Box::new(Ast::IntLit(1)),
                            InfixOp::Equals
                        )),
                        vec![Ast::Return(Box::new(Ast::IntLit(1)))],
                        None
                    ),
                    Ast::Return(Box::new(Ast::InfixExpr(
                        Box::new(Ast::FuncCall(
                            Box::new("fib".to_string()),
                            vec![Ast::InfixExpr(
                                Box::new(Ast::VarRef(Box::new("n".to_string()))),
                                Box::new(Ast::IntLit(1)),
                                InfixOp::Minus,
                            )]
                        )),
                        Box::new(Ast::FuncCall(
                            Box::new("fib".to_string()),
                            vec![Ast::InfixExpr(
                                Box::new(Ast::VarRef(Box::new("n".to_string()))),
                                Box::new(Ast::IntLit(2)),
                                InfixOp::Minus,
                            )]
                        )),
                        InfixOp::Plus
                    )))
                ]
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::FuncCall(
                    Box::new("fib".to_string()),
                    vec![Ast::IntLit(5)]
                )]
            )
        ]
    )
}

#[test]
fn test_ast_gen_while() {
    setup_ast!(
        "let x = 0; while x < 10 { if x == 0{continue;} if x == 7{break;} x++;} x;",
        ast
    );
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(0))
            ),
            Ast::WhileStmt(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("x".to_string()))),
                    Box::new(Ast::IntLit(10)),
                    InfixOp::LessThan
                )),
                vec![
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("x".to_string()))),
                            Box::new(Ast::IntLit(0)),
                            InfixOp::Equals
                        )),
                        vec![Ast::Continue],
                        None
                    ),
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("x".to_string()))),
                            Box::new(Ast::IntLit(7)),
                            InfixOp::Equals
                        )),
                        vec![Ast::Break],
                        None
                    ),
                    Ast::VarReassign(
                        Box::new("x".to_string()),
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("x".to_string()))),
                            Box::new(Ast::IntLit(1)),
                            InfixOp::Plus
                        ))
                    ),
                ]
            ),
            Ast::VarRef(Box::new("x".to_string()))
        ]
    )
}

#[test]
fn test_ast_gen_str_concat() {
    setup_ast!(r#"let x = "1"; let y = str(x) + "1"; println(y);"#, ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Str,
                Box::new(Ast::StringLit(Box::new("1".to_string())))
            ),
            Ast::VarDec(
                Box::new("y".to_string()),
                TypeTok::Str,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::FuncCall(
                        Box::new("str".to_string()),
                        vec![Ast::VarRef(Box::new("x".to_string()))]
                    )),
                    Box::new(Ast::StringLit(Box::new("1".to_string()))),
                    InfixOp::Plus
                ))
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::VarRef(Box::new("y".to_string()))]
            )
        ]
    )
}

#[test]
fn test_ast_gen_float_infix() {
    setup_ast!("let pi = 3 + 0.1415; let e: float = 2.7 + 0.08;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("pi".to_string()),
                TypeTok::Float,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(3)),
                    Box::new(Ast::FloatLit(OrderedFloat(0.1415))),
                    InfixOp::Plus
                ))
            ),
            Ast::VarDec(
                Box::new("e".to_string()),
                TypeTok::Float,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::FloatLit(OrderedFloat(2.7))),
                    Box::new(Ast::FloatLit(OrderedFloat(0.08))),
                    InfixOp::Plus
                ))
            )
        ]
    )
}
#[test]
fn test_ast_gen_arr_lit() {
    setup_ast!("let arr: int[] = [1, 2 - 1, int(3.0)];", ast);

    assert_eq!(
        ast, 
        vec![

            Ast::VarDec(
                Box::new("arr".to_string()), 
                TypeTok::IntArr(1), 
                Box::new(
                    Ast::ArrLit(
                        TypeTok::IntArr(1), 
                        vec![
                            Ast::IntLit(1),
                            Ast::InfixExpr(
                                Box::new(Ast::IntLit(2)), 
                                Box::new(Ast::IntLit(1)), 
                                InfixOp::Minus
                            ),
                            Ast::FuncCall(
                                Box::new("int".to_string()), 
                                vec![Ast::FloatLit(OrderedFloat(3.0))]
                            )
                        ]
                    )
                )
            )
        ]
    )
}

#[test]
fn test_ast_gen_arr_reassign() {
    setup_ast!("let ao: bool[] = [true, false]; ao = [false];", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("ao".to_string()), 
                TypeTok::BoolArr(1), 
                Box::new(
                    Ast::ArrLit(
                        TypeTok::BoolArr(1), 
                        vec![
                            Ast::BoolLit(true),
                            Ast::BoolLit(false)
                        ]
                    )
                )
            ),
            Ast::VarReassign(
                Box::new("ao".to_string()), 
                Box::new(
                    Ast::ArrLit(
                        TypeTok::BoolArr(1), 
                        vec![Ast::BoolLit(false)]
                    )
                )
            )
        ]
    )
}

#[test]
fn test_ast_gen_arr_idx_ref() {
    setup_ast!("let a: int[] = [1, 2, 3, 4]; let b = a[0];", ast);

    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("a".to_string()), 
                TypeTok::IntArr(1), 
                Box::new(
                    Ast::ArrLit(
                        TypeTok::IntArr(1), 
                        vec![
                            Ast::IntLit(1),
                            Ast::IntLit(2),
                            Ast::IntLit(3),
                            Ast::IntLit(4)
                        ]
                    )
                )
            ),
            Ast::VarDec(
                Box::new("b".to_string()), 
                TypeTok::Int, 
                Box::new(
                    Ast::ArrRef(
                        Box::new("a".to_string()), 
                        vec![Ast::IntLit(0)]
                    )
                )
            )
        ]
    )
}

#[test]
fn test_ast_gen_arr_idx_reassign() {
    setup_ast!("let arr = [1.0, 1.1, 1.2, 1.3]; arr[1] = 1.7;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("arr".to_string()),
                TypeTok::FloatArr(1),
                Box::new(Ast::ArrLit(
                    TypeTok::FloatArr(1),
                    vec![
                        Ast::FloatLit(OrderedFloat(1.0)),
                        Ast::FloatLit(OrderedFloat(1.1)),
                        Ast::FloatLit(OrderedFloat(1.2)),
                        Ast::FloatLit(OrderedFloat(1.3))
                    ]
                ))
            ),
            Ast::ArrReassign(
                Box::new("arr".to_string()),
                vec![Ast::IntLit(1)],
                Box::new(Ast::FloatLit(OrderedFloat(1.7)))
            )
        ]
    )

}


#[test]
fn test_ast_gen_n_dimensional_arr_dec_and_reassign() {
    setup_ast!(r#"let arr: str[][] = [["hi", "bye"], ["goodbye"]]; arr[0][1] = "bye bye"; "#, ast);

    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("arr".to_string()),
                TypeTok::StrArr(2), 
                Box::new(Ast::ArrLit(
                    TypeTok::StrArr(2),
                    vec![
                        Ast::ArrLit(
                            TypeTok::StrArr(1),
                            vec![
                                Ast::StringLit(Box::new("hi".to_string())),
                                Ast::StringLit(Box::new("bye".to_string()))
                            ]
                        ),
                        Ast::ArrLit(
                            TypeTok::StrArr(1),
                            vec![Ast::StringLit(Box::new("goodbye".to_string()))]
                        )
                    ]
                ))
            ),
            Ast::ArrReassign(
                Box::new("arr".to_string()),
                vec![Ast::IntLit(0), Ast::IntLit(1)],
                Box::new(Ast::StringLit(Box::new("bye bye".to_string())))
            )
        ]
    )
}

#[test]
fn test_ast_gen_nd_arr_ref() {
    setup_ast!("let arr = [[2.3, 4.3], [0.2, 9.5]]; let x = arr[1][0];", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("arr".to_string()),
                TypeTok::FloatArr(2),
                Box::new(Ast::ArrLit(
                    TypeTok::FloatArr(2),
                    vec![
                        Ast::ArrLit(
                            TypeTok::FloatArr(1),
                            vec![
                                Ast::FloatLit(OrderedFloat(2.3)),
                                Ast::FloatLit(OrderedFloat(4.3))
                            ]
                        ),
                        Ast::ArrLit(
                            TypeTok::FloatArr(1),
                            vec![
                                Ast::FloatLit(OrderedFloat(0.2)),
                                Ast::FloatLit(OrderedFloat(9.5))
                            ]
                        )
                    ]
                ))
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Float,
                Box::new(Ast::ArrRef(
                    Box::new("arr".to_string()),
                    vec![Ast::IntLit(1), Ast::IntLit(0)]
                ))
            )
        ]
    )
}