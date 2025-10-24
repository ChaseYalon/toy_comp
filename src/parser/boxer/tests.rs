use super::{Boxer, TBox, Token};
use crate::{lexer::Lexer, token::TypeTok};
use ordered_float::OrderedFloat;
#[test]
fn test_boxer_int_literal() {
    let input = String::from("4");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(boxes, vec![TBox::Expr(vec![Token::IntLit(4)]),])
}

#[test]
fn test_boxer_infix_expression() {
    let input = String::from("8 - 3 * 5");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![TBox::Expr(vec![
            Token::IntLit(8),
            Token::Minus,
            Token::IntLit(3),
            Token::Multiply,
            Token::IntLit(5),
        ]),]
    )
}

#[test]
fn test_boxer_var_dec() {
    let input = String::from("let x = 9;");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![TBox::VarDec(
            Token::VarName(Box::new(String::from("x"))),
            None,
            vec![Token::IntLit(9)]
        )]
    )
}

#[test]
fn test_boxer_var_ref() {
    let input = "let x = 7; x = 8;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![
            TBox::VarDec(
                Token::VarName(Box::new("x".to_string())),
                None,
                vec![Token::IntLit(7)]
            ),
            TBox::VarReassign(
                Token::VarRef(Box::new("x".to_string())),
                vec![Token::IntLit(8)]
            )
        ]
    )
}

#[test]
fn test_boxer_static_type() {
    let input = "let foo: int = 9;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![TBox::VarDec(
            Token::VarName(Box::new("foo".to_string())),
            Some(TypeTok::Int),
            vec![Token::IntLit(9)]
        )]
    )
}

#[test]
fn test_boxer_bool_infix() {
    let input = "let x = 9 <= 4 || false;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![TBox::VarDec(
            Token::VarName(Box::new("x".to_string())),
            None,
            vec![
                Token::IntLit(9),
                Token::LessThanEqt,
                Token::IntLit(4),
                Token::Or,
                Token::BoolLit(false)
            ]
        )]
    )
}

#[test]
fn test_boxer_return_bool() {
    let input = "true || false;".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![TBox::Expr(vec![
            Token::BoolLit(true),
            Token::Or,
            Token::BoolLit(false),
        ])]
    )
}

#[test]
fn test_boxer_if_stmt() {
    let input = "let x: int = 5; if x < 9 {x = 6;}".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![
            TBox::VarDec(
                Token::VarName(Box::new("x".to_string())),
                Some(TypeTok::Int),
                vec![Token::IntLit(5)]
            ),
            TBox::IfStmt(
                vec![
                    Token::VarRef(Box::new("x".to_string())),
                    Token::LessThan,
                    Token::IntLit(9)
                ],
                vec![TBox::VarReassign(
                    Token::VarRef(Box::new("x".to_string())),
                    vec![Token::IntLit(6)],
                )],
                None
            )
        ]
    )
}

#[test]
fn test_boxer_nested_if() {
    let input = "if true{let x = 9; if x > 10 {x = 8;}}".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![TBox::IfStmt(
            vec![Token::BoolLit(true)],
            vec![
                TBox::VarDec(
                    Token::VarName(Box::new("x".to_string())),
                    None,
                    vec![Token::IntLit(9)]
                ),
                TBox::IfStmt(
                    vec![
                        Token::VarRef(Box::new("x".to_string())),
                        Token::GreaterThan,
                        Token::IntLit(10)
                    ],
                    vec![TBox::VarReassign(
                        Token::VarRef(Box::new("x".to_string())),
                        vec![Token::IntLit(8)]
                    )],
                    None
                )
            ],
            None
        )]
    )
}

#[test]
fn test_boxer_if_else() {
    let input = "if true && false{let x = 5;} else {let x: int = 6;}".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![TBox::IfStmt(
            vec![Token::BoolLit(true), Token::And, Token::BoolLit(false),],
            vec![TBox::VarDec(
                Token::VarName(Box::new("x".to_string())),
                None,
                vec![Token::IntLit(5)]
            )],
            Some(vec![TBox::VarDec(
                Token::VarName(Box::new("x".to_string())),
                Some(TypeTok::Int),
                vec![Token::IntLit(6)]
            )])
        )]
    )
}

#[test]
fn test_boxer_parens() {
    let input = "let x: int = (14 - 3 * (6/2));".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![TBox::VarDec(
            Token::VarName(Box::new("x".to_string())),
            Some(TypeTok::Int),
            vec![
                Token::LParen,
                Token::IntLit(14),
                Token::Minus,
                Token::IntLit(3),
                Token::Multiply,
                Token::LParen,
                Token::IntLit(6),
                Token::Divide,
                Token::IntLit(2),
                Token::RParen,
                Token::RParen
            ]
        )]
    )
}

#[test]
fn test_boxer_func_dec_and_call() {
    let input = "fn add(a: int, b: int): int {return a + b;} let x = add(2, 3);".to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![
            TBox::FuncDec(
                Token::VarName(Box::new("add".to_string())),
                vec![
                    TBox::FuncParam(Token::VarRef(Box::new("a".to_string())), TypeTok::Int),
                    TBox::FuncParam(Token::VarRef(Box::new("b".to_string())), TypeTok::Int,)
                ],
                TypeTok::Int,
                vec![TBox::Return(Box::new(TBox::Expr(vec![
                    Token::VarRef(Box::new("a".to_string())),
                    Token::Plus,
                    Token::VarRef(Box::new("b".to_string())),
                ])))]
            ),
            TBox::VarDec(
                Token::VarName(Box::new("x".to_string())),
                None,
                vec![
                    Token::VarRef(Box::new("add".to_string())),
                    Token::LParen,
                    Token::IntLit(2),
                    Token::Comma,
                    Token::IntLit(3),
                    Token::RParen
                ]
            )
        ]
    )
}

#[test]
fn test_boxer_string_lit() {
    let input = r#"let x: str = "hello world""#.to_string();
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes,
        vec![TBox::VarDec(
            Token::VarName(Box::new("x".to_string())),
            Some(TypeTok::Str),
            vec![Token::StringLit(Box::new("hello world".to_string()))]
        )]
    )
}
#[test]
fn test_boxer_while_loops() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks =
        l.lex("let x = 0; while x < 10{if x == 0{continue;} if x == 7{break;} x++;}x;".to_string());
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![
            TBox::VarDec(
                Token::VarName(Box::new("x".to_string())),
                None,
                vec![Token::IntLit(0)]
            ),
            TBox::While(
                vec![
                    Token::VarRef(Box::new("x".to_string())),
                    Token::LessThan,
                    Token::IntLit(10)
                ],
                vec![
                    TBox::IfStmt(
                        vec![
                            Token::VarRef(Box::new("x".to_string())),
                            Token::Equals,
                            Token::IntLit(0),
                        ],
                        vec![TBox::Continue],
                        None,
                    ),
                    TBox::IfStmt(
                        vec![
                            Token::VarRef(Box::new("x".to_string())),
                            Token::Equals,
                            Token::IntLit(7)
                        ],
                        vec![TBox::Break],
                        None
                    ),
                    TBox::VarReassign(
                        Token::VarRef(Box::new("x".to_string())),
                        vec![
                            Token::VarRef(Box::new("x".to_string())),
                            Token::Plus,
                            Token::IntLit(1),
                        ]
                    )
                ]
            ),
            TBox::Expr(vec![Token::VarRef(Box::new("x".to_string()))])
        ]
    )
}

#[test]
fn test_boxer_fn_loop() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("fn loop(): int{let x = 0; while x<10{if x == 1{x++; continue;} if x == 7{break} x++;} return x;} loop();".to_string());
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![
            TBox::FuncDec(
                Token::VarName(Box::new("loop".to_string())),
                vec![],
                TypeTok::Int,
                vec![
                    TBox::VarDec(
                        Token::VarName(Box::new("x".to_string())),
                        None,
                        vec![Token::IntLit(0)]
                    ),
                    TBox::While(
                        vec![
                            Token::VarRef(Box::new("x".to_string())),
                            Token::LessThan,
                            Token::IntLit(10)
                        ],
                        vec![
                            TBox::IfStmt(
                                vec![
                                    Token::VarRef(Box::new("x".to_string())),
                                    Token::Equals,
                                    Token::IntLit(1)
                                ],
                                vec![
                                    TBox::VarReassign(
                                        Token::VarRef(Box::new("x".to_string())),
                                        vec![
                                            Token::VarRef(Box::new("x".to_string())),
                                            Token::Plus,
                                            Token::IntLit(1)
                                        ]
                                    ),
                                    TBox::Continue
                                ],
                                None
                            ),
                            TBox::IfStmt(
                                vec![
                                    Token::VarRef(Box::new("x".to_string())),
                                    Token::Equals,
                                    Token::IntLit(7)
                                ],
                                vec![TBox::Break],
                                None
                            ),
                            TBox::VarReassign(
                                Token::VarRef(Box::new("x".to_string())),
                                vec![
                                    Token::VarRef(Box::new("x".to_string())),
                                    Token::Plus,
                                    Token::IntLit(1)
                                ]
                            )
                        ]
                    ),
                    TBox::Return(Box::new(TBox::Expr(vec![Token::VarRef(Box::new(
                        "x".to_string()
                    ))])))
                ]
            ),
            TBox::Expr(vec![
                Token::VarRef(Box::new("loop".to_string())),
                Token::LParen,
                Token::RParen
            ])
        ]
    );
}

#[test]
fn test_boxer_fn_no_params() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("fn foo(): int{ return 1;} foo();".to_string());
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![
            TBox::FuncDec(
                Token::VarName(Box::new("foo".to_string())),
                vec![],
                TypeTok::Int,
                vec![TBox::Return(Box::new(TBox::Expr(vec![Token::IntLit(1)])))]
            ),
            TBox::Expr(vec![
                Token::VarRef(Box::new("foo".to_string())),
                Token::LParen,
                Token::RParen
            ])
        ]
    )
}

#[test]
fn test_boxer_float() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("let x = 3.14159; let y: float = 9.3;".to_string());
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![
            TBox::VarDec(
                Token::VarName(Box::new("x".to_string())),
                None,
                vec![Token::FloatLit(OrderedFloat(3.14159))]
            ),
            TBox::VarDec(
                Token::VarName(Box::new("y".to_string())),
                Some(TypeTok::Float),
                vec![Token::FloatLit(OrderedFloat(9.3))]
            )
        ]
    )
}

#[test]
fn test_boxer_arr_lit() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(r#"let arr: str[] = ["foo", "bar"];"#.to_string());
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![
            TBox::VarDec(
                Token::VarName(Box::new("arr".to_string())), 
                Some(TypeTok::StrArr), 
                vec![
                    Token::LBrack,
                    Token::StringLit(Box::new("foo".to_string())),
                    Token::Comma,
                    Token::StringLit(Box::new("bar".to_string())),
                    Token::RBrack
                ]
            )
        ]
    )
}

#[test]
fn test_boxer_arr_item_reassign() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("let arr = [1, 2, 3]; arr[1] = 4;".to_string());
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes,
        vec![
            TBox::VarDec(
                Token::VarName(Box::new("arr".to_string())),
                None,
                vec![
                    Token::LBrack,
                    Token::IntLit(1),
                    Token::Comma,
                    Token::IntLit(2),
                    Token::Comma,
                    Token::IntLit(3),
                    Token::RBrack
                ]
            ),
            TBox::ArrReassign(
                Token::VarRef(Box::new("arr".to_string())),
                vec![Token::IntLit(1)],
                vec![Token::IntLit(4)]
            )
        ]
    )
}