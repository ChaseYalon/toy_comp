use crate::parser::ast::InfixOp;
use crate::{
    lexer::Lexer,
    parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer},
    token::TypeTok,
};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

fn eq_ast_ignoring_src(x: &Ast, y: &Ast) -> bool {
    match (x, y) {
        (Ast::IntLit(xi), Ast::IntLit(yi)) => xi == yi,
        (Ast::BoolLit(xb), Ast::BoolLit(yb)) => xb == yb,
        (Ast::FloatLit(xf), Ast::FloatLit(yf)) => xf == yf,
        (Ast::Break, Ast::Break) => true,
        (Ast::Continue, Ast::Continue) => true,

        (Ast::InfixExpr(xl, xr, xo, _), Ast::InfixExpr(yl, yr, yo, _)) => {
            xo == yo && eq_ast_ignoring_src(xl, yl) && eq_ast_ignoring_src(xr, yr)
        }

        (Ast::EmptyExpr(xc, _), Ast::EmptyExpr(yc, _)) => eq_ast_ignoring_src(xc, yc),

        (Ast::VarDec(xn, xt, xv, _), Ast::VarDec(yn, yt, yv, _)) => {
            xn == yn && xt == yt && eq_ast_ignoring_src(xv, yv)
        }

        (Ast::VarRef(xn, _), Ast::VarRef(yn, _)) => xn == yn,

        (Ast::IfStmt(xc, xb, xa, _), Ast::IfStmt(yc, yb, ya, _)) => {
            eq_ast_ignoring_src(xc, yc)
                && compare_ast_vecs(xb.clone(), yb.clone())
                && match (xa, ya) {
                    (None, None) => true,
                    (Some(xav), Some(yav)) => compare_ast_vecs(xav.clone(), yav.clone()),
                    _ => false,
                }
        }

        (Ast::FuncParam(xn, xt, _), Ast::FuncParam(yn, yt, _)) => xn == yn && xt == yt,

        (Ast::FuncDec(xn, xp, xr, xb, _), Ast::FuncDec(yn, yp, yr, yb, _)) => {
            xn == yn
                && xr == yr
                && compare_ast_vecs(xp.clone(), yp.clone())
                && compare_ast_vecs(xb.clone(), yb.clone())
        }

        (Ast::FuncCall(xn, xp, _), Ast::FuncCall(yn, yp, _)) => {
            xn == yn && compare_ast_vecs(xp.clone(), yp.clone())
        }

        (Ast::Return(xv, _), Ast::Return(yv, _)) => eq_ast_ignoring_src(xv, yv),

        (Ast::StringLit(xs, _), Ast::StringLit(ys, _)) => xs == ys,

        (Ast::WhileStmt(xc, xb, _), Ast::WhileStmt(yc, yb, _)) => {
            eq_ast_ignoring_src(xc, yc) && compare_ast_vecs(xb.clone(), yb.clone())
        }

        (Ast::ArrLit(xt, xv, _), Ast::ArrLit(yt, yv, _)) => {
            xt == yt && compare_ast_vecs(xv.clone(), yv.clone())
        }

        (Ast::IndexAccess(xt, xi, _), Ast::IndexAccess(yt, yi, _)) => {
            eq_ast_ignoring_src(xt, yt) && eq_ast_ignoring_src(xi, yi)
        }

        (Ast::StructInterface(xn, xkv, _), Ast::StructInterface(yn, ykv, _)) => {
            xn == yn && xkv == ykv
        }

        (Ast::StructLit(xn, xkv, _), Ast::StructLit(yn, ykv, _)) => {
            if xn != yn {
                return false;
            }
            if xkv.len() != ykv.len() {
                return false;
            }
            xkv.iter().all(|(k, (xast, xt))| {
                if let Some((yast, yt)) = ykv.get(k) {
                    xt == yt && eq_ast_ignoring_src(xast, yast)
                } else {
                    false
                }
            })
        }

        (Ast::MemberAccess(xt, xm, _), Ast::MemberAccess(yt, ym, _)) => {
            eq_ast_ignoring_src(xt, yt) && xm == ym
        }

        (Ast::Not(xn), Ast::Not(yn)) => eq_ast_ignoring_src(xn, yn),

        (Ast::Assignment(xl, xr, _), Ast::Assignment(yl, yr, _)) => {
            eq_ast_ignoring_src(xl, yl) && eq_ast_ignoring_src(xr, yr)
        }

        _ => todo!("Chase you have not implemented {} node yet", x.node_type()),
    }
}

fn compare_ast_vecs(a: Vec<Ast>, b: Vec<Ast>) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.iter()
        .zip(b.iter())
        .all(|(x, y)| eq_ast_ignoring_src(x, y))
}

macro_rules! setup_ast {
    ($i: expr, $ast: ident) => {
        let input = $i.to_string();
        let mut l = Lexer::new();
        let mut b = Boxer::new();
        let mut a = AstGenerator::new();

        let toks = l.lex(input).unwrap();
        let boxes = b.box_toks(toks).unwrap();
        let $ast = a.generate(boxes).unwrap();
    };
}

#[test]
fn test_ast_gen_int_literal() {
    setup_ast!("64", ast);
    assert!(compare_ast_vecs(ast, vec![Ast::IntLit(64)]))
}

#[test]
fn test_ast_gen_infix_exprs() {
    setup_ast!("18 - 3", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(18)),
            Box::new(Ast::IntLit(3)),
            InfixOp::Minus,
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_order_ops() {
    setup_ast!("18 - 3 * 5", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(18)),
            Box::new(Ast::InfixExpr(
                Box::new(Ast::IntLit(3)),
                Box::new(Ast::IntLit(5)),
                InfixOp::Multiply,
                "".to_string()
            )),
            InfixOp::Minus,
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_var_dec() {
    setup_ast!("let x = 9;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::IntLit(9)),
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_var_reassign() {
    setup_ast!("let x = 9; x = 5;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9)),
                "".to_string()
            ),
            Ast::Assignment(
                Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                Box::new(Ast::IntLit(5)),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_static_type() {
    setup_ast!("let x: int = 9;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::IntLit(9)),
            "".to_string()
        )]
    ))
}
#[test]
fn test_ast_gen_bool_lit() {
    setup_ast!("let x: bool = false;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Bool,
            Box::new(Ast::BoolLit(false)),
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_bool_infix() {
    setup_ast!(
        "let foo = 8 > 4 || false; let x = 9; let bar = 9 == x;",
        ast
    );

    assert!(compare_ast_vecs(
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
                        "".to_string()
                    )),
                    Box::new(Ast::BoolLit(false)),
                    InfixOp::Or,
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9)),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("bar".to_string()),
                TypeTok::Bool,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(9)),
                    Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                    InfixOp::Equals,
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_mixed_bool_int() {
    setup_ast!("let x = 4 + 3 < 6;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Bool,
            Box::new(Ast::InfixExpr(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(4)),
                    Box::new(Ast::IntLit(3)),
                    InfixOp::Plus,
                    "".to_string()
                )),
                Box::new(Ast::IntLit(6)),
                InfixOp::LessThan,
                "".to_string()
            )),
            "".to_string()
        )]
    ))
}
#[test]
fn test_asg_gen_modulo() {
    setup_ast!("5 % 3;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(5)),
            Box::new(Ast::IntLit(3)),
            InfixOp::Modulo,
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_return_bool() {
    setup_ast!("let x: bool = true; x || false;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::BoolLit(true)),
                "".to_string()
            ),
            Ast::InfixExpr(
                Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                Box::new(Ast::BoolLit(false)),
                InfixOp::Or,
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_if_stmt() {
    setup_ast!("let x = false; if x || true {x = true;}", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::BoolLit(false)),
                "".to_string()
            ),
            Ast::IfStmt(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                    Box::new(Ast::BoolLit(true)),
                    InfixOp::Or,
                    "".to_string()
                )),
                vec![Ast::Assignment(
                    Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                    Box::new(Ast::BoolLit(true)),
                    "".to_string()
                )],
                None,
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_if_stmt_complex() {
    setup_ast!("let x:int = 8; if true {x = 4}; x;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(8)),
                "".to_string()
            ),
            Ast::IfStmt(
                Box::new(Ast::BoolLit(true)),
                vec![Ast::Assignment(
                    Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                    Box::new(Ast::IntLit(4)),
                    "".to_string()
                )],
                None,
                "".to_string()
            ),
            Ast::VarRef(Box::new("x".to_string()), "".to_string()),
        ]
    ))
}

#[test]
fn test_ast_gen_if_else() {
    setup_ast!("if true && false {let x = 7;} else {let x = 8;}", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::IfStmt(
            Box::new(Ast::InfixExpr(
                Box::new(Ast::BoolLit(true)),
                Box::new(Ast::BoolLit(false)),
                InfixOp::And,
                "".to_string()
            )),
            vec![Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(7)),
                "".to_string()
            )],
            Some(vec![Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(8)),
                "".to_string()
            )]),
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_nested_parens() {
    setup_ast!("let x = (5 * (3 + 4)) / 7;", ast);
    //This nesting is a crime against humanity
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::InfixExpr(
                Box::new(Ast::EmptyExpr(
                    Box::new(Ast::InfixExpr(
                        Box::new(Ast::IntLit(5)),
                        Box::new(Ast::EmptyExpr(
                            Box::new(Ast::InfixExpr(
                                Box::new(Ast::IntLit(3)),
                                Box::new(Ast::IntLit(4)),
                                InfixOp::Plus,
                                "".to_string()
                            )),
                            "".to_string()
                        )),
                        InfixOp::Multiply,
                        "".to_string()
                    )),
                    "".to_string()
                )),
                Box::new(Ast::IntLit(7)),
                InfixOp::Divide,
                "".to_string()
            )),
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_consecutive_parens() {
    setup_ast!("let x = (5 + 2) + (3 + 1);", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::InfixExpr(
                Box::new(Ast::EmptyExpr(
                    Box::new(Ast::InfixExpr(
                        Box::new(Ast::IntLit(5)),
                        Box::new(Ast::IntLit(2)),
                        InfixOp::Plus,
                        "".to_string()
                    )),
                    "".to_string()
                )),
                Box::new(Ast::EmptyExpr(
                    Box::new(Ast::InfixExpr(
                        Box::new(Ast::IntLit(3)),
                        Box::new(Ast::IntLit(1)),
                        InfixOp::Plus,
                        "".to_string()
                    )),
                    "".to_string()
                )),
                InfixOp::Plus,
                "".to_string()
            )),
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_func_dec_call() {
    setup_ast!(
        "fn add(a: int, b: int): int{return a + b;} let x = add(2, 3);",
        ast
    );

    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::FuncDec(
                Box::new("add".to_string()),
                vec![
                    Ast::FuncParam(Box::new("a".to_string()), TypeTok::Int, "".to_string()),
                    Ast::FuncParam(Box::new("b".to_string()), TypeTok::Int, "".to_string())
                ],
                TypeTok::Int,
                vec![Ast::Return(
                    Box::new(Ast::InfixExpr(
                        Box::new(Ast::VarRef(Box::new("a".to_string()), "".to_string())),
                        Box::new(Ast::VarRef(Box::new("b".to_string()), "".to_string())),
                        InfixOp::Plus,
                        "".to_string()
                    )),
                    "".to_string()
                )],
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::FuncCall(
                    Box::new("add".to_string()),
                    vec![Ast::IntLit(2), Ast::IntLit(3),],
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_str_lit_and_concatenation() {
    setup_ast!(
        r#"let x: str = "hello "; let y = "world"; let z = x + y;"#,
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Str,
                Box::new(Ast::StringLit(
                    Box::new("hello ".to_string()),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("y".to_string()),
                TypeTok::Str,
                Box::new(Ast::StringLit(
                    Box::new("world".to_string()),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("z".to_string()),
                TypeTok::Str,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                    Box::new(Ast::VarRef(Box::new("y".to_string()), "".to_string())),
                    InfixOp::Plus,
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_print_int_bool() {
    setup_ast!("println(true); println(1);", ast);

    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::BoolLit(true)],
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::IntLit(1),],
                "".to_string()
            )
        ]
    ))
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
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::FuncDec(
                Box::new("fib".to_string()),
                vec![Ast::FuncParam(
                    Box::new("n".to_string()),
                    TypeTok::Int,
                    "".to_string()
                )],
                TypeTok::Int,
                vec![
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("n".to_string()), "".to_string())),
                            Box::new(Ast::IntLit(0)),
                            InfixOp::Equals,
                            "".to_string()
                        )),
                        vec![Ast::Return(Box::new(Ast::IntLit(0)), "".to_string())],
                        None,
                        "".to_string()
                    ),
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("n".to_string()), "".to_string())),
                            Box::new(Ast::IntLit(1)),
                            InfixOp::Equals,
                            "".to_string()
                        )),
                        vec![Ast::Return(Box::new(Ast::IntLit(1)), "".to_string())],
                        None,
                        "".to_string()
                    ),
                    Ast::Return(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::FuncCall(
                                Box::new("fib".to_string()),
                                vec![Ast::InfixExpr(
                                    Box::new(Ast::VarRef(
                                        Box::new("n".to_string()),
                                        "".to_string()
                                    )),
                                    Box::new(Ast::IntLit(1)),
                                    InfixOp::Minus,
                                    "".to_string()
                                )],
                                "".to_string()
                            )),
                            Box::new(Ast::FuncCall(
                                Box::new("fib".to_string()),
                                vec![Ast::InfixExpr(
                                    Box::new(Ast::VarRef(
                                        Box::new("n".to_string()),
                                        "".to_string()
                                    )),
                                    Box::new(Ast::IntLit(2)),
                                    InfixOp::Minus,
                                    "".to_string()
                                )],
                                "".to_string()
                            )),
                            InfixOp::Plus,
                            "".to_string()
                        )),
                        "".to_string()
                    )
                ],
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::FuncCall(
                    Box::new("fib".to_string()),
                    vec![Ast::IntLit(5)],
                    "".to_string()
                )],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_while() {
    setup_ast!(
        "let x = 0; while x < 10 { if x == 0{continue;} if x == 7{break;} x++;} x;",
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(0)),
                "".to_string()
            ),
            Ast::WhileStmt(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                    Box::new(Ast::IntLit(10)),
                    InfixOp::LessThan,
                    "".to_string()
                )),
                vec![
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                            Box::new(Ast::IntLit(0)),
                            InfixOp::Equals,
                            "".to_string()
                        )),
                        vec![Ast::Continue],
                        None,
                        "".to_string()
                    ),
                    Ast::IfStmt(
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                            Box::new(Ast::IntLit(7)),
                            InfixOp::Equals,
                            "".to_string()
                        )),
                        vec![Ast::Break],
                        None,
                        "".to_string()
                    ),
                    Ast::Assignment(
                        Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                            Box::new(Ast::IntLit(1)),
                            InfixOp::Plus,
                            "".to_string()
                        )),
                        "".to_string()
                    ),
                ],
                "".to_string()
            ),
            Ast::VarRef(Box::new("x".to_string()), "".to_string())
        ]
    ))
}

#[test]
fn test_ast_gen_str_concat() {
    setup_ast!(r#"let x = "1"; let y = str(x) + "1"; println(y);"#, ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Str,
                Box::new(Ast::StringLit(Box::new("1".to_string()), "".to_string())),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("y".to_string()),
                TypeTok::Str,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::FuncCall(
                        Box::new("str".to_string()),
                        vec![Ast::VarRef(Box::new("x".to_string()), "".to_string())],
                        "".to_string()
                    )),
                    Box::new(Ast::StringLit(Box::new("1".to_string()), "".to_string())),
                    InfixOp::Plus,
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::VarRef(Box::new("y".to_string()), "".to_string())],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_float_infix() {
    setup_ast!("let pi = 3 + 0.1415; let e: float = 2.7 + 0.08;", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("pi".to_string()),
                TypeTok::Float,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(3)),
                    Box::new(Ast::FloatLit(OrderedFloat(0.1415))),
                    InfixOp::Plus,
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("e".to_string()),
                TypeTok::Float,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::FloatLit(OrderedFloat(2.7))),
                    Box::new(Ast::FloatLit(OrderedFloat(0.08))),
                    InfixOp::Plus,
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}
#[test]
fn test_ast_gen_arr_lit() {
    setup_ast!("let arr: int[] = [1, 2 - 1, int(3.0)];", ast);

    assert!(compare_ast_vecs(
        ast,
        vec![Ast::VarDec(
            Box::new("arr".to_string()),
            TypeTok::IntArr(1),
            Box::new(Ast::ArrLit(
                TypeTok::IntArr(1),
                vec![
                    Ast::IntLit(1),
                    Ast::InfixExpr(
                        Box::new(Ast::IntLit(2)),
                        Box::new(Ast::IntLit(1)),
                        InfixOp::Minus,
                        "".to_string()
                    ),
                    Ast::FuncCall(
                        Box::new("int".to_string()),
                        vec![Ast::FloatLit(OrderedFloat(3.0))],
                        "".to_string()
                    )
                ],
                "".to_string()
            )),
            "".to_string()
        )]
    ))
}

#[test]
fn test_ast_gen_arr_reassign() {
    setup_ast!("let ao: bool[] = [true, false]; ao = [false];", ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("ao".to_string()),
                TypeTok::BoolArr(1),
                Box::new(Ast::ArrLit(
                    TypeTok::BoolArr(1),
                    vec![Ast::BoolLit(true), Ast::BoolLit(false)],
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::Assignment(
                Box::new(Ast::VarRef(Box::new("ao".to_string()), "".to_string())),
                Box::new(Ast::ArrLit(
                    TypeTok::BoolArr(1),
                    vec![Ast::BoolLit(false)],
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_arr_idx_ref() {
    setup_ast!("let a: int[] = [1, 2, 3, 4]; let b = a[0];", ast);

    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("a".to_string()),
                TypeTok::IntArr(1),
                Box::new(Ast::ArrLit(
                    TypeTok::IntArr(1),
                    vec![
                        Ast::IntLit(1),
                        Ast::IntLit(2),
                        Ast::IntLit(3),
                        Ast::IntLit(4)
                    ],
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("b".to_string()),
                TypeTok::Int,
                Box::new(Ast::IndexAccess(
                    Box::new(Ast::VarRef(Box::new("a".to_string()), "".to_string())),
                    Box::new(Ast::IntLit(0)),
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_arr_idx_reassign() {
    setup_ast!("let arr = [1.0, 1.1, 1.2, 1.3]; arr[1] = 1.7;", ast);
    assert!(compare_ast_vecs(
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
                    ],
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::Assignment(
                Box::new(Ast::IndexAccess(
                    Box::new(Ast::VarRef(Box::new("arr".to_string()), "".to_string())),
                    Box::new(Ast::IntLit(1)),
                    "".to_string()
                )),
                Box::new(Ast::FloatLit(OrderedFloat(1.7))),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_n_dimensional_arr_dec_and_reassign() {
    setup_ast!(
        r#"let arr: str[][] = [["hi", "bye"], ["goodbye"]]; arr[0][1] = "bye bye"; "#,
        ast
    );

    assert!(compare_ast_vecs(
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
                                Ast::StringLit(Box::new("hi".to_string()), "".to_string()),
                                Ast::StringLit(Box::new("bye".to_string()), "".to_string())
                            ],
                            "".to_string()
                        ),
                        Ast::ArrLit(
                            TypeTok::StrArr(1),
                            vec![Ast::StringLit(
                                Box::new("goodbye".to_string()),
                                "".to_string()
                            )],
                            "".to_string()
                        )
                    ],
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::Assignment(
                Box::new(Ast::IndexAccess(
                    Box::new(Ast::IndexAccess(
                        Box::new(Ast::VarRef(Box::new("arr".to_string()), "".to_string())),
                        Box::new(Ast::IntLit(0)),
                        "".to_string()
                    )),
                    Box::new(Ast::IntLit(1)),
                    "".to_string()
                )),
                Box::new(Ast::StringLit(
                    Box::new("bye bye".to_string()),
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_nd_arr_ref() {
    setup_ast!(
        "let arr = [[2.3, 4.3], [0.2, 9.5]]; let x = arr[1][0];",
        ast
    );
    assert!(compare_ast_vecs(
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
                            ],
                            "".to_string()
                        ),
                        Ast::ArrLit(
                            TypeTok::FloatArr(1),
                            vec![
                                Ast::FloatLit(OrderedFloat(0.2)),
                                Ast::FloatLit(OrderedFloat(9.5))
                            ],
                            "".to_string()
                        )
                    ],
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Float,
                Box::new(Ast::IndexAccess(
                    Box::new(Ast::IndexAccess(
                        Box::new(Ast::VarRef(Box::new("arr".to_string()), "".to_string())),
                        Box::new(Ast::IntLit(1)),
                        "".to_string()
                    )),
                    Box::new(Ast::IntLit(0)),
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_struct_def_and_ref() {
    setup_ast!(
        r#"struct Name{first: str, last: str}; let me = Name{first: "Chase", last: "Yalon"}; println(me.first);"#,
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Name".to_string()),
                Box::new(BTreeMap::from([
                    ("first".to_string(), TypeTok::Str),
                    ("last".to_string(), TypeTok::Str)
                ])),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("me".to_string()),
                TypeTok::Struct(BTreeMap::from([
                    ("first".to_string(), Box::new(TypeTok::Str)),
                    ("last".to_string(), Box::new(TypeTok::Str)),
                ])),
                Box::new(Ast::StructLit(
                    Box::new("Name".to_string()),
                    Box::new(BTreeMap::from([
                        (
                            "first".to_string(),
                            (
                                Ast::StringLit(Box::new("Chase".to_string()), "".to_string()),
                                TypeTok::Str
                            )
                        ),
                        (
                            "last".to_string(),
                            (
                                Ast::StringLit(Box::new("Yalon".to_string()), "".to_string()),
                                TypeTok::Str
                            )
                        )
                    ])),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::MemberAccess(
                    Box::new(Ast::VarRef(Box::new("me".to_string()), "".to_string())),
                    "first".to_string(),
                    "".to_string()
                )],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_struct_buggy() {
    setup_ast!(
        r#"struct Foo{fee: int, baz: bool}; let x = Foo{fee: 2, baz: false}; println(x.fee);"#,
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([
                    ("fee".to_string(), TypeTok::Int),
                    ("baz".to_string(), TypeTok::Bool)
                ])),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Struct(BTreeMap::from([
                    ("fee".to_string(), Box::new(TypeTok::Int)),
                    ("baz".to_string(), Box::new(TypeTok::Bool)),
                ])),
                Box::new(Ast::StructLit(
                    Box::new("Foo".to_string()),
                    Box::new(BTreeMap::from([
                        ("fee".to_string(), (Ast::IntLit(2), TypeTok::Int)),
                        ("baz".to_string(), (Ast::BoolLit(false), TypeTok::Bool))
                    ])),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::MemberAccess(
                    Box::new(Ast::VarRef(Box::new("x".to_string()), "".to_string())),
                    "fee".to_string(),
                    "".to_string()
                )],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_nested_struct() {
    setup_ast!(r#"struct Name{first: str, last: str}; struct Person{name: Name, age: int}; let me = Person{name: Name{first: "Chase", last: "Yalon"}, age: 15}; println(me.name.last);"#.to_string(), ast);
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Name".to_string()),
                Box::new(BTreeMap::from([
                    ("first".to_string(), TypeTok::Str),
                    ("last".to_string(), TypeTok::Str)
                ])),
                "".to_string()
            ),
            Ast::StructInterface(
                Box::new("Person".to_string()),
                Box::new(BTreeMap::from([
                    (
                        "name".to_string(),
                        TypeTok::Struct(BTreeMap::from([
                            ("first".to_string(), Box::new(TypeTok::Str)),
                            ("last".to_string(), Box::new(TypeTok::Str))
                        ]))
                    ),
                    ("age".to_string(), TypeTok::Int)
                ])),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("me".to_string()),
                TypeTok::Struct(BTreeMap::from([
                    (
                        "name".to_string(),
                        Box::new(TypeTok::Struct(BTreeMap::from([
                            ("first".to_string(), Box::new(TypeTok::Str)),
                            ("last".to_string(), Box::new(TypeTok::Str))
                        ])))
                    ),
                    ("age".to_string(), Box::new(TypeTok::Int))
                ]),),
                Box::new(Ast::StructLit(
                    Box::new("Person".to_string()),
                    Box::new(BTreeMap::from([
                        (
                            "name".to_string(),
                            (
                                Ast::StructLit(
                                    Box::new("Name".to_string()),
                                    Box::new(BTreeMap::from([
                                        (
                                            "first".to_string(),
                                            (
                                                Ast::StringLit(
                                                    Box::new("Chase".to_string()),
                                                    "".to_string()
                                                ),
                                                TypeTok::Str
                                            )
                                        ),
                                        (
                                            "last".to_string(),
                                            (
                                                Ast::StringLit(
                                                    Box::new("Yalon".to_string()),
                                                    "".to_string()
                                                ),
                                                TypeTok::Str
                                            )
                                        )
                                    ])),
                                    "".to_string()
                                ),
                                TypeTok::Struct(BTreeMap::from([
                                    ("first".to_string(), Box::new(TypeTok::Str)),
                                    ("last".to_string(), Box::new(TypeTok::Str))
                                ]))
                            )
                        ),
                        ("age".to_string(), (Ast::IntLit(15), TypeTok::Int))
                    ])),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::MemberAccess(
                    Box::new(Ast::MemberAccess(
                        Box::new(Ast::VarRef(Box::new("me".to_string()), "".to_string())),
                        "name".to_string(),
                        "".to_string()
                    )),
                    "last".to_string(),
                    "".to_string()
                )],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_struct_reassign() {
    setup_ast!(
        r#"
        struct Foo{bar: int};
        struct Baz{foo: Foo};
        struct Qux{baz: Baz};
        let a = Qux{baz: Baz{foo: Foo{bar: 1}}};
        a.baz.foo = Foo{bar: 2};
        "#,
        ast
    );

    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("bar".to_string(), TypeTok::Int)])),
                "".to_string()
            ),
            Ast::StructInterface(
                Box::new("Baz".to_string()),
                Box::new(BTreeMap::from([(
                    "foo".to_string(),
                    TypeTok::Struct(BTreeMap::from([(
                        "bar".to_string(),
                        Box::new(TypeTok::Int)
                    )]))
                )])),
                "".to_string()
            ),
            Ast::StructInterface(
                Box::new("Qux".to_string()),
                Box::new(BTreeMap::from([(
                    "baz".to_string(),
                    TypeTok::Struct(BTreeMap::from([(
                        "foo".to_string(),
                        Box::new(TypeTok::Struct(BTreeMap::from([(
                            "bar".to_string(),
                            Box::new(TypeTok::Int)
                        )])))
                    )]))
                )])),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("a".to_string()),
                TypeTok::Struct(BTreeMap::from([(
                    "baz".to_string(),
                    Box::new(TypeTok::Struct(BTreeMap::from([(
                        "foo".to_string(),
                        Box::new(TypeTok::Struct(BTreeMap::from([(
                            "bar".to_string(),
                            Box::new(TypeTok::Int)
                        )])))
                    )])))
                )])),
                Box::new(Ast::StructLit(
                    Box::new("Qux".to_string()),
                    Box::new(BTreeMap::from([(
                        "baz".to_string(),
                        (
                            Ast::StructLit(
                                Box::new("Baz".to_string()),
                                Box::new(BTreeMap::from([(
                                    "foo".to_string(),
                                    (
                                        Ast::StructLit(
                                            Box::new("Foo".to_string()),
                                            Box::new(BTreeMap::from([(
                                                "bar".to_string(),
                                                (Ast::IntLit(1), TypeTok::Int)
                                            )])),
                                            "".to_string()
                                        ),
                                        TypeTok::Struct(BTreeMap::from([(
                                            "bar".to_string(),
                                            Box::new(TypeTok::Int)
                                        )]))
                                    )
                                )])),
                                "".to_string()
                            ),
                            TypeTok::Struct(BTreeMap::from([(
                                "foo".to_string(),
                                Box::new(TypeTok::Struct(BTreeMap::from([(
                                    "bar".to_string(),
                                    Box::new(TypeTok::Int)
                                )])))
                            )]))
                        )
                    )])),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::Assignment(
                Box::new(Ast::MemberAccess(
                    Box::new(Ast::MemberAccess(
                        Box::new(Ast::VarRef(Box::new("a".to_string()), "".to_string())),
                        "baz".to_string(),
                        "".to_string()
                    )),
                    "foo".to_string(),
                    "".to_string()
                )),
                Box::new(Ast::StructLit(
                    Box::new("Foo".to_string()),
                    Box::new(BTreeMap::from([(
                        "bar".to_string(),
                        (Ast::IntLit(2), TypeTok::Int)
                    )])),
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_struct_func_param() {
    setup_ast!(
        "struct Foo{a: int}; fn bar(f: Foo): int{return f.a;} bar(Foo{a: 1});",
        ast
    );

    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)])),
                "".to_string()
            ),
            Ast::FuncDec(
                Box::new("bar".to_string()),
                vec![Ast::FuncParam(
                    Box::new("f".to_string()),
                    TypeTok::Struct(BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))])),
                    "".to_string()
                )],
                TypeTok::Int,
                vec![Ast::Return(
                    Box::new(Ast::MemberAccess(
                        Box::new(Ast::VarRef(Box::new("f".to_string()), "".to_string())),
                        "a".to_string(),
                        "".to_string()
                    )),
                    "".to_string()
                )],
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("bar".to_string()),
                vec![Ast::StructLit(
                    Box::new("Foo".to_string()),
                    Box::new(BTreeMap::from([(
                        "a".to_string(),
                        (Ast::IntLit(1), TypeTok::Int)
                    )])),
                    "".to_string()
                )],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_not() {
    setup_ast!(
        r#"let x = false || false; if !x{println("duh")} else {println("something has gone wrong")}"#,
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::BoolLit(false)),
                    Box::new(Ast::BoolLit(false)),
                    InfixOp::Or,
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::IfStmt(
                Box::new(Ast::Not(Box::new(Ast::VarRef(
                    Box::new("x".to_string()),
                    "".to_string()
                )))),
                vec![Ast::FuncCall(
                    Box::new("println".to_string()),
                    vec![Ast::StringLit(Box::new("duh".to_string()), "".to_string())],
                    "".to_string()
                )],
                Some(vec![Ast::FuncCall(
                    Box::new("println".to_string()),
                    vec![Ast::StringLit(
                        Box::new("something has gone wrong".to_string()),
                        "".to_string()
                    )],
                    "".to_string()
                )]),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_arr_ref_bug() {
    setup_ast!("let arr = [1, 2, 3]; arr[2] = 9; let x = arr[1] + 3;", ast);

    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::VarDec(
                Box::new("arr".to_string()),
                TypeTok::IntArr(1),
                Box::new(Ast::ArrLit(
                    TypeTok::IntArr(1),
                    vec![Ast::IntLit(1), Ast::IntLit(2), Ast::IntLit(3)],
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::Assignment(
                Box::new(Ast::IndexAccess(
                    Box::new(Ast::VarRef(Box::new("arr".to_string()), "".to_string())),
                    Box::new(Ast::IntLit(2)),
                    "".to_string()
                )),
                Box::new(Ast::IntLit(9)),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IndexAccess(
                        Box::new(Ast::VarRef(Box::new("arr".to_string()), "".to_string())),
                        Box::new(Ast::IntLit(1)),
                        "".to_string()
                    )),
                    Box::new(Ast::IntLit(3)),
                    InfixOp::Plus,
                    "".to_string()
                )),
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_nested_func_call_bug() {
    setup_ast!(
        "fn add(a: int, b: int): int {return a + b } println(add(5, 4));",
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::FuncDec(
                Box::new("add".to_string()),
                vec![
                    Ast::FuncParam(Box::new("a".to_string()), TypeTok::Int, "".to_string()),
                    Ast::FuncParam(Box::new("b".to_string()), TypeTok::Int, "".to_string())
                ],
                TypeTok::Int,
                vec![Ast::Return(
                    Box::new(Ast::InfixExpr(
                        Box::new(Ast::VarRef(Box::new("a".to_string()), "".to_string())),
                        Box::new(Ast::VarRef(Box::new("b".to_string()), "".to_string())),
                        InfixOp::Plus,
                        "".to_string()
                    )),
                    "".to_string()
                )],
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::FuncCall(
                    Box::new("add".to_string()),
                    vec![Ast::IntLit(5), Ast::IntLit(4)],
                    "".to_string()
                )],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_struct_func_call() {
    setup_ast!(
        r#"struct Human{name: str, age: int} for Human {fn set_age(n: int) {this.age = n }} let me = Human{name: "Chase", age: 16}; me.set_age(17);"#,
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Human".to_string()),
                Box::new(BTreeMap::from([
                    ("name".to_string(), TypeTok::Str),
                    ("age".to_string(), TypeTok::Int),
                ])),
                "".to_string()
            ),
            Ast::FuncDec(
                Box::new("Human:::set_age".to_string()),
                vec![
                    Ast::FuncParam(
                        Box::new("this".to_string()),
                        TypeTok::Struct(BTreeMap::from([
                            ("name".to_string(), Box::new(TypeTok::Str)),
                            ("age".to_string(), Box::new(TypeTok::Int)),
                        ])),
                        "".to_string()
                    ),
                    Ast::FuncParam(Box::new("n".to_string()), TypeTok::Int, "".to_string()),
                ],
                TypeTok::Void,
                vec![Ast::Assignment(
                    Box::new(Ast::MemberAccess(
                        Box::new(Ast::VarRef(Box::new("this".to_string()), "".to_string())),
                        "age".to_string(),
                        "".to_string()
                    )),
                    Box::new(Ast::VarRef(Box::new("n".to_string()), "".to_string())),
                    "".to_string()
                )],
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("me".to_string()),
                TypeTok::Struct(BTreeMap::from([
                    ("name".to_string(), Box::new(TypeTok::Str)),
                    ("age".to_string(), Box::new(TypeTok::Int)),
                ])),
                Box::new(Ast::StructLit(
                    Box::new("Human".to_string()),
                    Box::new(BTreeMap::from([
                        (
                            "name".to_string(),
                            (
                                Ast::StringLit(Box::new("Chase".to_string()), "".to_string()),
                                TypeTok::Str,
                            ),
                        ),
                        ("age".to_string(), (Ast::IntLit(16), TypeTok::Int)),
                    ])),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("Human:::set_age".to_string()),
                vec![
                    Ast::VarRef(Box::new("me".to_string()), "".to_string()),
                    Ast::IntLit(17)
                ],
                "".to_string()
            ),
        ]
    ))
}

#[test]
fn test_ast_gen_struct_func_call_multi_param() {
    setup_ast!(
        r#"
        struct Point{
            x: int,
            y: int,
        }
        for Point{
            fn move(dx: int, dy: int){
                this.x = dx;
                this.y = dy;
            }
        }

        let origin = Point{x: 0, y: 0};
        origin.move(2, 2);
        "#,
        ast
    );

    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Point".to_string()),
                Box::new(BTreeMap::from([
                    ("x".to_string(), TypeTok::Int),
                    ("y".to_string(), TypeTok::Int),
                ])),
                "".to_string()
            ),
            Ast::FuncDec(
                Box::new("Point:::move".to_string()),
                vec![
                    Ast::FuncParam(
                        Box::new("this".to_string()),
                        TypeTok::Struct(BTreeMap::from([
                            ("x".to_string(), Box::new(TypeTok::Int)),
                            ("y".to_string(), Box::new(TypeTok::Int)),
                        ])),
                        "".to_string()
                    ),
                    Ast::FuncParam(Box::new("dx".to_string()), TypeTok::Int, "".to_string()),
                    Ast::FuncParam(Box::new("dy".to_string()), TypeTok::Int, "".to_string()),
                ],
                TypeTok::Void,
                vec![
                    Ast::Assignment(
                        Box::new(Ast::MemberAccess(
                            Box::new(Ast::VarRef(Box::new("this".to_string()), "".to_string())),
                            "x".to_string(),
                            "".to_string()
                        )),
                        Box::new(Ast::VarRef(Box::new("dx".to_string()), "".to_string())),
                        "".to_string()
                    ),
                    Ast::Assignment(
                        Box::new(Ast::MemberAccess(
                            Box::new(Ast::VarRef(Box::new("this".to_string()), "".to_string())),
                            "y".to_string(),
                            "".to_string()
                        )),
                        Box::new(Ast::VarRef(Box::new("dy".to_string()), "".to_string())),
                        "".to_string()
                    )
                ],
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("origin".to_string()),
                TypeTok::Struct(BTreeMap::from([
                    ("x".to_string(), Box::new(TypeTok::Int)),
                    ("y".to_string(), Box::new(TypeTok::Int)),
                ])),
                Box::new(Ast::StructLit(
                    Box::new("Point".to_string()),
                    Box::new(BTreeMap::from([
                        ("x".to_string(), (Ast::IntLit(0), TypeTok::Int)),
                        ("y".to_string(), (Ast::IntLit(0), TypeTok::Int)),
                    ])),
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("Point:::move".to_string()),
                vec![
                    Ast::VarRef(Box::new("origin".to_string()), "".to_string()),
                    Ast::IntLit(2),
                    Ast::IntLit(2)
                ],
                "".to_string()
            )
        ]
    ))
}

#[test]
fn test_ast_gen_struct_arr_func_call() {
    setup_ast!(
        r#"
        struct Foo{
            a: int
        }
        for Foo {
            fn set_a(new_a: int){
                this.a = new_a;
            }
        }
        let fee = [
            Foo{
                a: 4
            }
        ];

        let i = 0;
        while i < len(fee){
            fee[i].set_a(5);
            i++;
        }
        println(fee[0].a)
        "#,
        ast
    );
    assert!(compare_ast_vecs(
        ast,
        vec![
            Ast::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)])),
                "".to_string()
            ),
            Ast::FuncDec(
                Box::new("Foo:::set_a".to_string()),
                vec![
                    Ast::FuncParam(
                        Box::new("this".to_string()),
                        TypeTok::Struct(BTreeMap::from([(
                            "a".to_string(),
                            Box::new(TypeTok::Int)
                        )])),
                        "".to_string()
                    ),
                    Ast::FuncParam(Box::new("new_a".to_string()), TypeTok::Int, "".to_string())
                ],
                TypeTok::Void,
                vec![Ast::Assignment(
                    Box::new(Ast::MemberAccess(
                        Box::new(Ast::VarRef(Box::new("this".to_string()), "".to_string())),
                        "a".to_string(),
                        "".to_string()
                    )),
                    Box::new(Ast::VarRef(Box::new("new_a".to_string()), "".to_string())),
                    "".to_string()
                )],
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("fee".to_string()),
                TypeTok::StructArr(
                    BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))]),
                    1
                ),
                Box::new(Ast::ArrLit(
                    TypeTok::StructArr(
                        BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))]),
                        1
                    ),
                    vec![Ast::StructLit(
                        Box::new("Foo".to_string()),
                        Box::new(BTreeMap::from([(
                            "a".to_string(),
                            (Ast::IntLit(4), TypeTok::Int)
                        )])),
                        "".to_string()
                    )],
                    "".to_string()
                )),
                "".to_string()
            ),
            Ast::VarDec(
                Box::new("i".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(0)),
                "".to_string()
            ),
            Ast::WhileStmt(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("i".to_string()), "".to_string())),
                    Box::new(Ast::FuncCall(
                        Box::new("len".to_string()),
                        vec![Ast::VarRef(Box::new("fee".to_string()), "".to_string())],
                        "".to_string()
                    )),
                    InfixOp::LessThan,
                    "".to_string()
                )),
                vec![
                    Ast::FuncCall(
                        Box::new("Foo:::set_a".to_string()),
                        vec![
                            Ast::IndexAccess(
                                Box::new(Ast::VarRef(Box::new("fee".to_string()), "".to_string())),
                                Box::new(Ast::VarRef(Box::new("i".to_string()), "".to_string())),
                                "".to_string()
                            ),
                            Ast::IntLit(5)
                        ],
                        "".to_string()
                    ),
                    Ast::Assignment(
                        Box::new(Ast::VarRef(Box::new("i".to_string()), "".to_string())),
                        Box::new(Ast::InfixExpr(
                            Box::new(Ast::VarRef(Box::new("i".to_string()), "".to_string())),
                            Box::new(Ast::IntLit(1)),
                            InfixOp::Plus,
                            "".to_string()
                        )),
                        "".to_string()
                    )
                ],
                "".to_string()
            ),
            Ast::FuncCall(
                Box::new("println".to_string()),
                vec![Ast::MemberAccess(
                    Box::new(Ast::IndexAccess(
                        Box::new(Ast::VarRef(Box::new("fee".to_string()), "".to_string())),
                        Box::new(Ast::IntLit(0)),
                        "".to_string()
                    )),
                    "a".to_string(),
                    "".to_string()
                )],
                "".to_string()
            )
        ]
    ))
}
