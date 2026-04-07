use super::{Boxer, TBox, Token};
use crate::{
    errors::Span,
    lexer::Lexer,
    token::{ExternType, QualifiedExternType, SpannedToken, TypeTok},
};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

fn eq_sp_tok_vec(a: &Vec<SpannedToken>, b: &Vec<SpannedToken>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| x.tok == y.tok)
}

fn eq_tbox_ignoring_src(x: &TBox, y: &TBox) -> bool {
    match (x, y) {
        (TBox::Break(_), TBox::Break(_)) => true,
        (TBox::Continue(_), TBox::Continue(_)) => true,

        (TBox::Expr(xv, _), TBox::Expr(yv, _)) => eq_sp_tok_vec(xv, yv),

        (TBox::VarDec(xn, xt, xv, _), TBox::VarDec(yn, yt, yv, _)) => {
            xn.tok == yn.tok && xt == yt && eq_sp_tok_vec(xv, yv)
        }
        (TBox::IfStmt(xc, xb, xe, xa, _), TBox::IfStmt(yc, yb, ye, ya, _)) => {
            eq_sp_tok_vec(xc, yc)
                && compare_tbox_vecs(xb.clone(), yb.clone())
                && match (xe, ye) {
                    (Some(xev), Some(yev)) => {
                        if xev.len() != yev.len() {
                            false
                        } else {
                            xev.iter().zip(yev.iter()).all(|((xc, xb), (yc, yb))| {
                                eq_sp_tok_vec(xc, yc) && compare_tbox_vecs(xb.clone(), yb.clone())
                            })
                        }
                    }
                    (None, None) => true,
                    _ => false,
                }
                && match (xa, ya) {
                    (None, None) => true,
                    (Some(xav), Some(yav)) => compare_tbox_vecs(xav.clone(), yav.clone()),
                    _ => false,
                }
        }

        (TBox::FuncParam(xn, xt, _), TBox::FuncParam(yn, yt, _)) => xn.tok == yn.tok && xt == yt,
        (TBox::ExternFuncParam(xn, xt, _), TBox::ExternFuncParam(yn, yt, _)) => {
            xn.tok == yn.tok && xt == yt
        }

        (TBox::FuncDec(xn, xp, xr, xb, _, xie), TBox::FuncDec(yn, yp, yr, yb, _, yie)) => {
            xn.tok == yn.tok
                && xr == yr
                && compare_tbox_vecs(xp.clone(), yp.clone())
                && compare_tbox_vecs(xb.clone(), yb.clone())
                && xie == yie
        }

        (TBox::Return(xv, _), TBox::Return(yv, _)) => eq_tbox_ignoring_src(xv, yv),

        (TBox::While(xc, xb, _), TBox::While(yc, yb, _)) => {
            eq_sp_tok_vec(xc, yc) && compare_tbox_vecs(xb.clone(), yb.clone())
        }
        (TBox::Assign(xl, xr, _), TBox::Assign(yl, yr, _)) => {
            eq_sp_tok_vec(xl, yl) && eq_sp_tok_vec(xr, yr)
        }
        (TBox::StructInterface(xn, xkv, _), TBox::StructInterface(yn, ykv, _)) => {
            xn == yn && xkv == ykv
        }
        (TBox::ExternFuncDec(xn, xp, xr, _), TBox::ExternFuncDec(yn, yp, yr, _)) => {
            xn.tok == yn.tok && xr == yr && compare_tbox_vecs(xp.clone(), yp.clone())
        }
        (TBox::ImportStmt(xn, _), TBox::ImportStmt(yn, _)) => xn == yn,
        (TBox::Interface(xt, _), TBox::Interface(yt, _)) => xt == yt,
        _ => false,
    }
}
fn compare_tbox_vecs(a: Vec<TBox>, b: Vec<TBox>) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.iter()
        .zip(b.iter())
        .all(|(x, y)| eq_tbox_ignoring_src(x, y))
}
#[test]
fn test_boxer_int_literal() {
    let input = String::from("4");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::Expr(
            vec![SpannedToken::new_null(Token::IntLit(4))],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_infix_expression() {
    let input = String::from("8 - 3 * 5");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::Expr(
            vec![
                SpannedToken::new_null(Token::IntLit(8)),
                SpannedToken::new_null(Token::Minus),
                SpannedToken::new_null(Token::IntLit(3)),
                SpannedToken::new_null(Token::Multiply),
                SpannedToken::new_null(Token::IntLit(5)),
            ],
            Span::null_span()
        ),]
    ))
}

#[test]
fn test_boxer_var_dec() {
    let input = String::from("let x = 9;");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::VarDec(
            SpannedToken::new_null(Token::VarName(Box::new(String::from("x")))),
            None,
            vec![SpannedToken::new_null(Token::IntLit(9))],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_var_ref() {
    let input = "let x = 7; x = 8;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                None,
                vec![SpannedToken::new_null(Token::IntLit(7))],
                Span::null_span()
            ),
            TBox::Assign(
                vec![SpannedToken::new_null(Token::VarRef(Box::new(
                    "x".to_string()
                )))],
                vec![SpannedToken::new_null(Token::IntLit(8))],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_static_type() {
    let input = "let foo: int = 9;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::VarDec(
            SpannedToken::new_null(Token::VarName(Box::new("foo".to_string()))),
            Some(TypeTok::Int),
            vec![SpannedToken::new_null(Token::IntLit(9))],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_bool_infix() {
    let input = "let x = 9 <= 4 || false;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::VarDec(
            SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
            None,
            vec![
                SpannedToken::new_null(Token::IntLit(9)),
                SpannedToken::new_null(Token::LessThanEqt),
                SpannedToken::new_null(Token::IntLit(4)),
                SpannedToken::new_null(Token::Or),
                SpannedToken::new_null(Token::BoolLit(false))
            ],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_return_bool() {
    let input = "true || false;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::Expr(
            vec![
                SpannedToken::new_null(Token::BoolLit(true)),
                SpannedToken::new_null(Token::Or),
                SpannedToken::new_null(Token::BoolLit(false)),
            ],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_if_stmt() {
    let input = "let x: int = 5; if x < 9 {x = 6;}".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                Some(TypeTok::Int),
                vec![SpannedToken::new_null(Token::IntLit(5))],
                Span::null_span()
            ),
            TBox::IfStmt(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::LessThan),
                    SpannedToken::new_null(Token::IntLit(9)),
                ],
                vec![TBox::Assign(
                    vec![SpannedToken::new_null(Token::VarRef(Box::new(
                        "x".to_string()
                    )))],
                    vec![SpannedToken::new_null(Token::IntLit(6))],
                    Span::null_span()
                )],
                None,
                None,
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_nested_if() {
    let input = "if true{let x = 9; if x > 10 {x = 8;}}".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::IfStmt(
            vec![SpannedToken::new_null(Token::BoolLit(true))],
            vec![
                TBox::VarDec(
                    SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                    None,
                    vec![SpannedToken::new_null(Token::IntLit(9))],
                    Span::null_span()
                ),
                TBox::IfStmt(
                    vec![
                        SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                        SpannedToken::new_null(Token::GreaterThan),
                        SpannedToken::new_null(Token::IntLit(10)),
                    ],
                    vec![TBox::Assign(
                        vec![SpannedToken::new_null(Token::VarRef(Box::new(
                            "x".to_string()
                        )))],
                        vec![SpannedToken::new_null(Token::IntLit(8))],
                        Span::null_span()
                    )],
                    None,
                    None,
                    Span::null_span()
                )
            ],
            None,
            None,
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_if_else() {
    let input = "if true && false{let x = 5;} else {let x: int = 6;}".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::IfStmt(
            vec![
                SpannedToken::new_null(Token::BoolLit(true)),
                SpannedToken::new_null(Token::And),
                SpannedToken::new_null(Token::BoolLit(false)),
            ],
            vec![TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                None,
                vec![SpannedToken::new_null(Token::IntLit(5))],
                Span::null_span()
            )],
            None,
            Some(vec![TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                Some(TypeTok::Int),
                vec![SpannedToken::new_null(Token::IntLit(6))],
                Span::null_span()
            )]),
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_parens() {
    let input = "let x: int = (14 - 3 * (6/2));".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::VarDec(
            SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
            Some(TypeTok::Int),
            vec![
                SpannedToken::new_null(Token::LParen),
                SpannedToken::new_null(Token::IntLit(14)),
                SpannedToken::new_null(Token::Minus),
                SpannedToken::new_null(Token::IntLit(3)),
                SpannedToken::new_null(Token::Multiply),
                SpannedToken::new_null(Token::LParen),
                SpannedToken::new_null(Token::IntLit(6)),
                SpannedToken::new_null(Token::Divide),
                SpannedToken::new_null(Token::IntLit(2)),
                SpannedToken::new_null(Token::RParen),
                SpannedToken::new_null(Token::RParen)
            ],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_func_dec_and_call() {
    let input = "fn add(a: int, b: int): int {return a + b;} let x = add(2, 3);".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::FuncDec(
                SpannedToken::new_null(Token::VarName(Box::new("add_int_int".to_string()))),
                vec![
                    TBox::FuncParam(
                        SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                        TypeTok::Int,
                        Span::null_span()
                    ),
                    TBox::FuncParam(
                        SpannedToken::new_null(Token::VarRef(Box::new("b".to_string()))),
                        TypeTok::Int,
                        Span::null_span()
                    )
                ],
                TypeTok::Int,
                vec![TBox::Return(
                    Box::new(TBox::Expr(
                        vec![
                            SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                            SpannedToken::new_null(Token::Plus),
                            SpannedToken::new_null(Token::VarRef(Box::new("b".to_string()))),
                        ],
                        Span::null_span()
                    )),
                    Span::null_span()
                )],
                Span::null_span(),
                false
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("add".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::IntLit(2)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::IntLit(3)),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_string_lit() {
    let input = r#"let x: str = "hello world""#.to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::VarDec(
            SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
            Some(TypeTok::Str),
            vec![SpannedToken::new_null(Token::StringLit(Box::new(
                "hello world".to_string()
            )))],
            Span::null_span()
        )]
    ))
}
#[test]
fn test_boxer_while_loops() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l
        .lex("let x = 0; while x < 10{if x == 0{continue;} if x == 7{break;} x++;}x;".to_string())
        .unwrap();
    let boxes = b.box_toks(toks);
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                None,
                vec![SpannedToken::new_null(Token::IntLit(0))],
                Span::null_span()
            ),
            TBox::While(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::LessThan),
                    SpannedToken::new_null(Token::IntLit(10))
                ],
                vec![
                    TBox::IfStmt(
                        vec![
                            SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                            SpannedToken::new_null(Token::Equals),
                            SpannedToken::new_null(Token::IntLit(0)),
                        ],
                        vec![TBox::Continue(Span::null_span())],
                        None,
                        None,
                        Span::null_span()
                    ),
                    TBox::IfStmt(
                        vec![
                            SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                            SpannedToken::new_null(Token::Equals),
                            SpannedToken::new_null(Token::IntLit(7))
                        ],
                        vec![TBox::Break(Span::null_span())],
                        None,
                        None,
                        Span::null_span()
                    ),
                    TBox::Assign(
                        vec![SpannedToken::new_null(Token::VarRef(Box::new(
                            "x".to_string()
                        )))],
                        vec![
                            SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                            SpannedToken::new_null(Token::Plus),
                            SpannedToken::new_null(Token::IntLit(1)),
                        ],
                        Span::null_span()
                    ),
                ],
                Span::null_span()
            ),
            TBox::Expr(
                vec![SpannedToken::new_null(Token::VarRef(Box::new(
                    "x".to_string()
                )))],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_fn_loop() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("fn loop(): int{let x = 0; while x<10{if x == 1{x++; continue;} if x == 7{break} x++;} return x;} loop();".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::FuncDec(
                SpannedToken::new_null(Token::VarName(Box::new("loop".to_string()))),
                vec![],
                TypeTok::Int,
                vec![
                    TBox::VarDec(
                        SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                        None,
                        vec![SpannedToken::new_null(Token::IntLit(0))],
                        Span::null_span()
                    ),
                    TBox::While(
                        vec![
                            SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                            SpannedToken::new_null(Token::LessThan),
                            SpannedToken::new_null(Token::IntLit(10))
                        ],
                        vec![
                            TBox::IfStmt(
                                vec![
                                    SpannedToken::new_null(Token::VarRef(Box::new(
                                        "x".to_string()
                                    ))),
                                    SpannedToken::new_null(Token::Equals),
                                    SpannedToken::new_null(Token::IntLit(1))
                                ],
                                vec![
                                    TBox::Assign(
                                        vec![SpannedToken::new_null(Token::VarRef(Box::new(
                                            "x".to_string()
                                        )))],
                                        vec![
                                            SpannedToken::new_null(Token::VarRef(Box::new(
                                                "x".to_string()
                                            ))),
                                            SpannedToken::new_null(Token::Plus),
                                            SpannedToken::new_null(Token::IntLit(1))
                                        ],
                                        Span::null_span()
                                    ),
                                    TBox::Continue(Span::null_span())
                                ],
                                None,
                                None,
                                Span::null_span()
                            ),
                            TBox::IfStmt(
                                vec![
                                    SpannedToken::new_null(Token::VarRef(Box::new(
                                        "x".to_string()
                                    ))),
                                    SpannedToken::new_null(Token::Equals),
                                    SpannedToken::new_null(Token::IntLit(7))
                                ],
                                vec![TBox::Break(Span::null_span())],
                                None,
                                None,
                                Span::null_span()
                            ),
                            TBox::Assign(
                                vec![SpannedToken::new_null(Token::VarRef(Box::new(
                                    "x".to_string()
                                )))],
                                vec![
                                    SpannedToken::new_null(Token::VarRef(Box::new(
                                        "x".to_string()
                                    ))),
                                    SpannedToken::new_null(Token::Plus),
                                    SpannedToken::new_null(Token::IntLit(1))
                                ],
                                Span::null_span()
                            )
                        ],
                        Span::null_span()
                    ),
                    TBox::Return(
                        Box::new(TBox::Expr(
                            vec![SpannedToken::new_null(Token::VarRef(Box::new(
                                "x".to_string()
                            )))],
                            Span::null_span()
                        )),
                        Span::null_span()
                    )
                ],
                Span::null_span(),
                false
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("loop".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ));
}

#[test]
fn test_boxer_fn_no_params() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("fn foo(): int{ return 1;} foo();".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::FuncDec(
                SpannedToken::new_null(Token::VarName(Box::new("foo".to_string()))),
                vec![],
                TypeTok::Int,
                vec![TBox::Return(
                    Box::new(TBox::Expr(
                        vec![SpannedToken::new_null(Token::IntLit(1))],
                        Span::null_span()
                    )),
                    Span::null_span()
                )],
                Span::null_span(),
                false
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("foo".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span(),
            )
        ]
    ))
}

#[test]
fn test_boxer_float() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("let x = 3.14159; let y: float = 9.3;".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                None,
                vec![SpannedToken::new_null(Token::FloatLit(OrderedFloat(
                    3.14159
                )))],
                Span::null_span()
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("y".to_string()))),
                Some(TypeTok::Float),
                vec![SpannedToken::new_null(Token::FloatLit(OrderedFloat(9.3)))],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_arr_lit() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(r#"let arr: str[] = ["foo", "bar"];"#.to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::VarDec(
            SpannedToken::new_null(Token::VarName(Box::new("arr".to_string()))),
            Some(TypeTok::StrArr(1)),
            vec![
                SpannedToken::new_null(Token::LBrack),
                SpannedToken::new_null(Token::StringLit(Box::new("foo".to_string()))),
                SpannedToken::new_null(Token::Comma),
                SpannedToken::new_null(Token::StringLit(Box::new("bar".to_string()))),
                SpannedToken::new_null(Token::RBrack)
            ],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_arr_item_reassign() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("let arr = [1, 2, 3]; arr[1] = 4;".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("arr".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::IntLit(1)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::IntLit(2)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::IntLit(3)),
                    SpannedToken::new_null(Token::RBrack)
                ],
                Span::null_span()
            ),
            TBox::Assign(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("arr".to_string()))),
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::IntLit(1)),
                    SpannedToken::new_null(Token::RBrack)
                ],
                vec![SpannedToken::new_null(Token::IntLit(4))],
                Span::null_span()
            )
        ]
    ))
}
#[test]
fn test_boxer_n_dimensional_arr_reassign() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "let arr = [[true, true, false], [false, false, true]]; arr[0][1] = false;".to_string(),
    );
    let boxes = b.box_toks(toks.unwrap());

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("arr".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::BoolLit(true)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::BoolLit(true)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::BoolLit(false)),
                    SpannedToken::new_null(Token::RBrack),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::BoolLit(false)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::BoolLit(false)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::BoolLit(true)),
                    SpannedToken::new_null(Token::RBrack),
                    SpannedToken::new_null(Token::RBrack)
                ],
                Span::null_span()
            ),
            TBox::Assign(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("arr".to_string()))),
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::IntLit(0)),
                    SpannedToken::new_null(Token::RBrack),
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::IntLit(1)),
                    SpannedToken::new_null(Token::RBrack)
                ],
                vec![SpannedToken::new_null(Token::BoolLit(false))],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_struct_lit_and_ref() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "struct Point {x: float, y: float}; let a = Point{x: 0.0, y: 0.0}; println(a.x);"
            .to_string(),
    );
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Point".to_string()),
                Box::new(BTreeMap::from([
                    ("x".to_string(), TypeTok::Float),
                    ("y".to_string(), TypeTok::Float),
                ])),
                Span::null_span()
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("a".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("Point".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::FloatLit(OrderedFloat(0.0))),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::VarRef(Box::new("y".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::FloatLit(OrderedFloat(0.0))),
                    SpannedToken::new_null(Token::RBrace),
                ],
                Span::null_span()
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("println".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    SpannedToken::new_null(Token::Dot),
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_struct_problematic() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(r#"struct Name{first: str, last: str}; let me = Name{first: "Chase", last: "Yalon"}; println(me.first);"#.to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Name".to_string()),
                Box::new(BTreeMap::from([
                    ("first".to_string(), TypeTok::Str),
                    ("last".to_string(), TypeTok::Str),
                ])),
                Span::null_span()
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("me".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("Name".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("first".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::StringLit(Box::new("Chase".to_string()))),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::VarRef(Box::new("last".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::StringLit(Box::new("Yalon".to_string()))),
                    SpannedToken::new_null(Token::RBrace),
                ],
                Span::null_span()
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("println".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::VarRef(Box::new("me".to_string()))),
                    SpannedToken::new_null(Token::Dot),
                    SpannedToken::new_null(Token::VarRef(Box::new("first".to_string()))),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            ),
        ]
    ))
}

#[test]
fn test_boxer_nested_structs() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(r#"struct Name{first: str, last: str}; struct Person{name: Name, age: int}; let me = Person{name: Name{first: "Chase", last: "Yalon"}, age: 15};"#.to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Name".to_string()),
                Box::new(BTreeMap::from([
                    ("first".to_string(), TypeTok::Str),
                    ("last".to_string(), TypeTok::Str),
                ])),
                Span::null_span()
            ),
            TBox::StructInterface(
                Box::new("Person".to_string()),
                Box::new(BTreeMap::from([
                    (
                        "name".to_string(),
                        TypeTok::Struct(BTreeMap::from([
                            ("first".to_string(), Box::new(TypeTok::Str)),
                            ("last".to_string(), Box::new(TypeTok::Str)),
                        ]))
                    ),
                    ("age".to_string(), TypeTok::Int),
                ])),
                Span::null_span()
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("me".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("Person".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("name".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::VarRef(Box::new("Name".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("first".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::StringLit(Box::new("Chase".to_string()))),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::VarRef(Box::new("last".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::StringLit(Box::new("Yalon".to_string()))),
                    SpannedToken::new_null(Token::RBrace),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::VarRef(Box::new("age".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::IntLit(15)),
                    SpannedToken::new_null(Token::RBrace),
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_struct_reassign() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "struct Fee{a: int}; struct Foo{a: Fee}; let b = Foo{a: Fee{3}}; b.a = Fee{9};".to_string(),
    );
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Fee".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)])),
                Span::null_span()
            ),
            TBox::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([(
                    "a".to_string(),
                    TypeTok::Struct(BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))]))
                )])),
                Span::null_span()
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("b".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::VarRef(Box::new("Fee".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::IntLit(3)),
                    SpannedToken::new_null(Token::RBrace),
                    SpannedToken::new_null(Token::RBrace),
                ],
                Span::null_span()
            ),
            TBox::Assign(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("b".to_string()))),
                    SpannedToken::new_null(Token::Dot),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string())))
                ],
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("Fee".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::IntLit(9)),
                    SpannedToken::new_null(Token::RBrace),
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_struct_func_param() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks =
        l.lex("struct Foo{a: int}; fn bar(f: Foo): int{return f.a;} bar(Foo{1});".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)])),
                Span::null_span()
            ),
            TBox::FuncDec(
                SpannedToken::new_null(Token::VarName(Box::new("bar_struct".to_string()))),
                vec![TBox::FuncParam(
                    SpannedToken::new_null(Token::VarRef(Box::new("f".to_string()))),
                    TypeTok::Struct(BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))])),
                    Span::null_span()
                )],
                TypeTok::Int,
                vec![TBox::Return(
                    Box::new(TBox::Expr(
                        vec![
                            SpannedToken::new_null(Token::VarRef(Box::new("f".to_string()))),
                            SpannedToken::new_null(Token::Dot),
                            SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                        ],
                        Span::null_span()
                    )),
                    Span::null_span()
                )],
                Span::null_span(),
                false
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("bar".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::IntLit(1)),
                    SpannedToken::new_null(Token::RBrace),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_struct_method_conversion() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "struct Point{x: int, y: int}; for Point { fn print_point() { println(this.x) } } let me = Point{x: 0, y: 0}; me.print_point();"
            .to_string(),
    );
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Point".to_string()),
                Box::new(BTreeMap::from([
                    ("x".to_string(), TypeTok::Int),
                    ("y".to_string(), TypeTok::Int),
                ])),
                Span::null_span()
            ),
            TBox::FuncDec(
                SpannedToken::new_null(Token::VarName(Box::new(
                    "Point:::print_point_struct".to_string()
                ))),
                vec![TBox::FuncParam(
                    SpannedToken::new_null(Token::VarRef(Box::new("this".to_string()))),
                    TypeTok::Struct(BTreeMap::from([
                        ("x".to_string(), Box::new(TypeTok::Int)),
                        ("y".to_string(), Box::new(TypeTok::Int)),
                    ])),
                    Span::null_span()
                )],
                TypeTok::Void,
                vec![TBox::Expr(
                    vec![
                        SpannedToken::new_null(Token::VarRef(Box::new("println".to_string()))),
                        SpannedToken::new_null(Token::LParen),
                        SpannedToken::new_null(Token::VarRef(Box::new("this".to_string()))),
                        SpannedToken::new_null(Token::Dot),
                        SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                        SpannedToken::new_null(Token::RParen),
                    ],
                    Span::null_span()
                )],
                Span::null_span(),
                false
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("me".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("Point".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::IntLit(0)),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::VarRef(Box::new("y".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::IntLit(0)),
                    SpannedToken::new_null(Token::RBrace),
                ],
                Span::null_span()
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("me".to_string()))),
                    SpannedToken::new_null(Token::Dot),
                    SpannedToken::new_null(Token::VarRef(Box::new("print_point".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_compound_assignment() {
    let input = String::from("x += 1;");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::Assign(
            vec![SpannedToken::new_null(Token::VarRef(Box::new(
                "x".to_string()
            )))],
            vec![
                SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                SpannedToken::new_null(Token::Plus),
                SpannedToken::new_null(Token::IntLit(1))
            ],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_increment() {
    let input = String::from("x++;");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::Assign(
            vec![SpannedToken::new_null(Token::VarRef(Box::new(
                "x".to_string()
            )))],
            vec![
                SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                SpannedToken::new_null(Token::Plus),
                SpannedToken::new_null(Token::IntLit(1))
            ],
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_extern_function_declaration() {
    let input = String::from("extern fn printf(msg: c_char_ptr): int;");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::ExternFuncDec(
            SpannedToken::new_null(Token::VarName(Box::new("printf".to_string()))),
            vec![TBox::ExternFuncParam(
                SpannedToken::new_null(Token::VarRef(Box::new("msg".to_string()))),
                QualifiedExternType{ty: ExternType::c_char(1), is_released: true},
                Span::null_span()
            )],
            TypeTok::Int,
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_extern_function_declaration_void() {
    let input = String::from("extern fn puts(msg: retained c_char_ptr): void;");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::ExternFuncDec(
            SpannedToken::new_null(Token::VarName(Box::new("puts".to_string()))),
            vec![TBox::ExternFuncParam(
                SpannedToken::new_null(Token::VarRef(Box::new("msg".to_string()))),
                QualifiedExternType{ty: ExternType::c_char(1), is_released: false},
                Span::null_span()
            )],
            TypeTok::Void,
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_import_stmt() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("import std.math; println(math.abs(-5));".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![
            TBox::ImportStmt("std.math".to_string(), Span::null_span()),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("println".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::VarRef(Box::new("math".to_string()))),
                    SpannedToken::new_null(Token::Dot),
                    SpannedToken::new_null(Token::VarRef(Box::new("abs".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::IntLit(-5)),
                    SpannedToken::new_null(Token::RParen),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_if_else_chain() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks =
        l.lex("if x == 1 {let y = 2;} else if x == 2 {let y = 3;} else {let y = 4;}".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert!(compare_tbox_vecs(
        boxes.unwrap(),
        vec![TBox::IfStmt(
            vec![
                SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                SpannedToken::new_null(Token::Equals),
                SpannedToken::new_null(Token::IntLit(1))
            ],
            vec![TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("y".to_string()))),
                None,
                vec![SpannedToken::new_null(Token::IntLit(2))],
                Span::null_span()
            )],
            Some(vec![(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::Equals),
                    SpannedToken::new_null(Token::IntLit(2))
                ],
                vec![TBox::VarDec(
                    SpannedToken::new_null(Token::VarName(Box::new("y".to_string()))),
                    None,
                    vec![SpannedToken::new_null(Token::IntLit(3))],
                    Span::null_span()
                )]
            )]),
            Some(vec![TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("y".to_string()))),
                None,
                vec![SpannedToken::new_null(Token::IntLit(4))],
                Span::null_span()
            )]),
            Span::null_span()
        )]
    ))
}

#[test]
fn test_boxer_struct_type_annotation() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "struct Foo{a: int}; let x: Foo = Foo{a: 5}; let arr: Foo[] = [Foo{a: 3}, Foo{a: 4}];"
            .to_string(),
    );
    let boxes = b.box_toks(toks.unwrap()).unwrap();
    assert!(compare_tbox_vecs(
        boxes,
        vec![
            TBox::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)])),
                Span::null_span()
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                Some(TypeTok::Struct(BTreeMap::from([(
                    "a".to_string(),
                    Box::new(TypeTok::Int)
                )]))),
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::IntLit(5)),
                    SpannedToken::new_null(Token::RBrace),
                ],
                Span::null_span()
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("arr".to_string()))),
                Some(TypeTok::StructArr(
                    BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))]),
                    1
                )),
                vec![
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::IntLit(3)),
                    SpannedToken::new_null(Token::RBrace),
                    SpannedToken::new_null(Token::Comma),
                    SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                    SpannedToken::new_null(Token::LBrace),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    SpannedToken::new_null(Token::Colon),
                    SpannedToken::new_null(Token::IntLit(4)),
                    SpannedToken::new_null(Token::RBrace),
                    SpannedToken::new_null(Token::RBrack),
                ],
                Span::null_span()
            ),
        ]
    ))
}

#[test]
fn test_boxer_func_struct_ret() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "struct Foo{a: int}; fn test(): Foo {return Foo{a: 3};} let x = test(); println(x.a);"
            .to_string(),
    );
    let boxes = b.box_toks(toks.unwrap()).unwrap();
    assert!(compare_tbox_vecs(
        boxes,
        vec![
            TBox::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)])),
                Span::null_span()
            ),
            TBox::FuncDec(
                SpannedToken::new_null(Token::VarName(Box::new("test".to_string()))),
                vec![],
                TypeTok::Struct(BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))])),
                vec![TBox::Return(
                    Box::new(TBox::Expr(
                        vec![
                            SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                            SpannedToken::new_null(Token::LBrace),
                            SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                            SpannedToken::new_null(Token::Colon),
                            SpannedToken::new_null(Token::IntLit(3)),
                            SpannedToken::new_null(Token::RBrace),
                        ],
                        Span::null_span()
                    )),
                    Span::null_span()
                )],
                Span::null_span(),
                false
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("test".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("println".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::Dot),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_func_struct_arr_ret() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("struct Foo{a: int}; fn test(): Foo[] {return [Foo{a: 3}, Foo{a: 5}];} let x = test(); println(x[0].a);".to_string());
    let boxes = b.box_toks(toks.unwrap()).unwrap();
    assert!(compare_tbox_vecs(
        boxes,
        vec![
            TBox::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)])),
                Span::null_span()
            ),
            TBox::FuncDec(
                SpannedToken::new_null(Token::VarName(Box::new("test".to_string()))),
                vec![],
                TypeTok::StructArr(
                    BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))]),
                    1
                ),
                vec![TBox::Return(
                    Box::new(TBox::Expr(
                        vec![
                            SpannedToken::new_null(Token::LBrack),
                            SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                            SpannedToken::new_null(Token::LBrace),
                            SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                            SpannedToken::new_null(Token::Colon),
                            SpannedToken::new_null(Token::IntLit(3)),
                            SpannedToken::new_null(Token::RBrace),
                            SpannedToken::new_null(Token::Comma),
                            SpannedToken::new_null(Token::VarRef(Box::new("Foo".to_string()))),
                            SpannedToken::new_null(Token::LBrace),
                            SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                            SpannedToken::new_null(Token::Colon),
                            SpannedToken::new_null(Token::IntLit(5)),
                            SpannedToken::new_null(Token::RBrace),
                            SpannedToken::new_null(Token::RBrack),
                        ],
                        Span::null_span()
                    )),
                    Span::null_span()
                )],
                Span::null_span(),
                false
            ),
            TBox::VarDec(
                SpannedToken::new_null(Token::VarName(Box::new("x".to_string()))),
                None,
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("test".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            ),
            TBox::Expr(
                vec![
                    SpannedToken::new_null(Token::VarRef(Box::new("println".to_string()))),
                    SpannedToken::new_null(Token::LParen),
                    SpannedToken::new_null(Token::VarRef(Box::new("x".to_string()))),
                    SpannedToken::new_null(Token::LBrack),
                    SpannedToken::new_null(Token::IntLit(0)),
                    SpannedToken::new_null(Token::RBrack),
                    SpannedToken::new_null(Token::Dot),
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    SpannedToken::new_null(Token::RParen)
                ],
                Span::null_span()
            )
        ]
    ))
}

#[test]
fn test_boxer_export_function() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let input = "export fn add(a: int, b: int): int {return a + b;}".to_string();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks).unwrap();

    assert!(compare_tbox_vecs(
        boxes,
        vec![TBox::FuncDec(
            SpannedToken::new_null(Token::VarName(Box::new("add_int_int".to_string()))),
            vec![
                TBox::FuncParam(
                    SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                    TypeTok::Int,
                    Span::null_span()
                ),
                TBox::FuncParam(
                    SpannedToken::new_null(Token::VarRef(Box::new("b".to_string()))),
                    TypeTok::Int,
                    Span::null_span()
                )
            ],
            TypeTok::Int,
            vec![TBox::Return(
                Box::new(TBox::Expr(
                    vec![
                        SpannedToken::new_null(Token::VarRef(Box::new("a".to_string()))),
                        SpannedToken::new_null(Token::Plus),
                        SpannedToken::new_null(Token::VarRef(Box::new("b".to_string()))),
                    ],
                    Span::null_span()
                )),
                Span::null_span()
            )],
            Span::null_span(),
            true
        )]
    ));
}
