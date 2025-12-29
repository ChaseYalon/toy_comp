use super::{Boxer, TBox, Token};
use crate::{lexer::Lexer, token::TypeTok};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
#[test]
fn test_boxer_int_literal() {
    let input = String::from("4");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(boxes.unwrap(), vec![TBox::Expr(vec![Token::IntLit(4)]),])
}

#[test]
fn test_boxer_infix_expression() {
    let input = String::from("8 - 3 * 5");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l.lex(input).unwrap();
    let boxes = b.box_toks(toks);

    assert_eq!(
        boxes.unwrap(),
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
    let toks = l
        .lex("let x = 0; while x < 10{if x == 0{continue;} if x == 7{break;} x++;}x;".to_string())
        .unwrap();
    let boxes = b.box_toks(toks);
    assert_eq!(
        boxes.unwrap(),
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
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
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
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
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
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
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
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
        vec![TBox::VarDec(
            Token::VarName(Box::new("arr".to_string())),
            Some(TypeTok::StrArr(1)),
            vec![
                Token::LBrack,
                Token::StringLit(Box::new("foo".to_string())),
                Token::Comma,
                Token::StringLit(Box::new("bar".to_string())),
                Token::RBrack
            ]
        )]
    )
}

#[test]
fn test_boxer_arr_item_reassign() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex("let arr = [1, 2, 3]; arr[1] = 4;".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
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
                vec![vec![Token::IntLit(1)]],
                vec![Token::IntLit(4)]
            )
        ]
    )
}
#[test]
fn test_boxer_n_dimensional_arr_reassign() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "let arr = [[true, true, false], [false, false, true]]; arr[0][1] = false;".to_string(),
    );
    let boxes = b.box_toks(toks.unwrap());

    assert_eq!(
        boxes.unwrap(),
        vec![
            TBox::VarDec(
                Token::VarName(Box::new("arr".to_string())),
                None,
                vec![
                    Token::LBrack,
                    Token::LBrack,
                    Token::BoolLit(true),
                    Token::Comma,
                    Token::BoolLit(true),
                    Token::Comma,
                    Token::BoolLit(false),
                    Token::RBrack,
                    Token::Comma,
                    Token::LBrack,
                    Token::BoolLit(false),
                    Token::Comma,
                    Token::BoolLit(false),
                    Token::Comma,
                    Token::BoolLit(true),
                    Token::RBrack,
                    Token::RBrack
                ]
            ),
            TBox::ArrReassign(
                Token::VarRef(Box::new("arr".to_string())),
                vec![vec![Token::IntLit(0)], vec![Token::IntLit(1)]],
                vec![Token::BoolLit(false)]
            )
        ]
    )
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
    assert_eq!(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Point".to_string()),
                Box::new(BTreeMap::from([
                    ("x".to_string(), TypeTok::Float),
                    ("y".to_string(), TypeTok::Float),
                ]))
            ),
            TBox::VarDec(
                Token::VarName(Box::new("a".to_string())),
                None,
                vec![
                    Token::VarRef(Box::new("Point".to_string())),
                    Token::LBrace,
                    Token::VarRef(Box::new("x".to_string())),
                    Token::Colon,
                    Token::FloatLit(OrderedFloat(0.0)),
                    Token::Comma,
                    Token::VarRef(Box::new("y".to_string())),
                    Token::Colon,
                    Token::FloatLit(OrderedFloat(0.0)),
                    Token::RBrace,
                ]
            ),
            TBox::Expr(vec![
                Token::VarRef(Box::new("println".to_string())),
                Token::LParen,
                Token::StructRef(Box::new("a".to_string()), vec!["x".to_string()]),
                Token::RParen
            ])
        ]
    )
}

#[test]
fn test_boxer_struct_problematic() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(r#"struct Name{first: str, last: str}; let me = Name{first: "Chase", last: "Yalon"}; println(me.first);"#.to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Name".to_string()),
                Box::new(BTreeMap::from([
                    ("first".to_string(), TypeTok::Str),
                    ("last".to_string(), TypeTok::Str),
                ]))
            ),
            TBox::VarDec(
                Token::VarName(Box::new("me".to_string())),
                None,
                vec![
                    Token::VarRef(Box::new("Name".to_string())),
                    Token::LBrace,
                    Token::VarRef(Box::new("first".to_string())),
                    Token::Colon,
                    Token::StringLit(Box::new("Chase".to_string())),
                    Token::Comma,
                    Token::VarRef(Box::new("last".to_string())),
                    Token::Colon,
                    Token::StringLit(Box::new("Yalon".to_string())),
                    Token::RBrace,
                ]
            ),
            TBox::Expr(vec![
                Token::VarRef(Box::new("println".to_string())),
                Token::LParen,
                Token::StructRef(Box::new("me".to_string()), vec!["first".to_string()]),
                Token::RParen
            ])
        ]
    )
}

#[test]
fn test_boxer_nested_structs() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(r#"struct Name{first: str, last: str}; struct Person{name: Name, age: int}; let me = Person{name: Name{first: "Chase", last: "Yalon"}, age: 15};"#.to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Name".to_string()),
                Box::new(BTreeMap::from([
                    ("first".to_string(), TypeTok::Str),
                    ("last".to_string(), TypeTok::Str),
                ]))
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
                ]))
            ),
            TBox::VarDec(
                Token::VarName(Box::new("me".to_string())),
                None,
                vec![
                    Token::VarRef(Box::new("Person".to_string())),
                    Token::LBrace,
                    Token::VarRef(Box::new("name".to_string())),
                    Token::Colon,
                    Token::VarRef(Box::new("Name".to_string())),
                    Token::LBrace,
                    Token::VarRef(Box::new("first".to_string())),
                    Token::Colon,
                    Token::StringLit(Box::new("Chase".to_string())),
                    Token::Comma,
                    Token::VarRef(Box::new("last".to_string())),
                    Token::Colon,
                    Token::StringLit(Box::new("Yalon".to_string())),
                    Token::RBrace,
                    Token::Comma,
                    Token::VarRef(Box::new("age".to_string())),
                    Token::Colon,
                    Token::IntLit(15),
                    Token::RBrace,
                ]
            )
        ]
    )
}

#[test]
fn test_boxer_struct_reassign() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(
        "struct Fee{a: int}; struct Foo{a: Fee}; let b = Foo{a: Fee{3}}; b.a = Fee{9};".to_string(),
    );
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Fee".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)]))
            ),
            TBox::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([(
                    "a".to_string(),
                    TypeTok::Struct(BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))]))
                )]))
            ),
            TBox::VarDec(
                Token::VarName(Box::new("b".to_string())),
                None,
                vec![
                    Token::VarRef(Box::new("Foo".to_string())),
                    Token::LBrace,
                    Token::VarRef(Box::new("a".to_string())),
                    Token::Colon,
                    Token::VarRef(Box::new("Fee".to_string())),
                    Token::LBrace,
                    Token::IntLit(3),
                    Token::RBrace,
                    Token::RBrace,
                ]
            ),
            TBox::StructReassign(
                Box::new("b".to_string()),
                vec!["a".to_string()],
                vec![
                    Token::VarRef(Box::new("Fee".to_string())),
                    Token::LBrace,
                    Token::IntLit(9),
                    Token::RBrace,
                ]
            )
        ]
    )
}

#[test]
fn test_boxer_struct_func_param() {
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks =
        l.lex("struct Foo{a: int}; fn bar(f: Foo): int{return f.a;} bar(Foo{1});".to_string());
    let boxes = b.box_toks(toks.unwrap());
    assert_eq!(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Foo".to_string()),
                Box::new(BTreeMap::from([("a".to_string(), TypeTok::Int)]))
            ),
            TBox::FuncDec(
                Token::VarName(Box::new("bar".to_string())),
                vec![TBox::FuncParam(
                    Token::VarRef(Box::new("f".to_string())),
                    TypeTok::Struct(BTreeMap::from([("a".to_string(), Box::new(TypeTok::Int))]))
                )],
                TypeTok::Int,
                vec![TBox::Return(Box::new(TBox::Expr(vec![Token::StructRef(
                    Box::new("f".to_string()),
                    vec!["a".to_string()]
                )])))]
            ),
            TBox::Expr(vec![
                Token::VarRef(Box::new("bar".to_string())),
                Token::LParen,
                Token::VarRef(Box::new("Foo".to_string())),
                Token::LBrace,
                Token::IntLit(1),
                Token::RBrace,
                Token::RParen
            ])
        ]
    )
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
    assert_eq!(
        boxes.unwrap(),
        vec![
            TBox::StructInterface(
                Box::new("Point".to_string()),
                Box::new(BTreeMap::from([
                    ("x".to_string(), TypeTok::Int),
                    ("y".to_string(), TypeTok::Int),
                ]))
            ),
            TBox::FuncDec(
                Token::VarName(Box::new("Point:::print_point".to_string())),
                vec![TBox::FuncParam(
                    Token::VarRef(Box::new("this".to_string())),
                    TypeTok::Struct(BTreeMap::from([
                        ("x".to_string(), Box::new(TypeTok::Int)),
                        ("y".to_string(), Box::new(TypeTok::Int)),
                    ]))
                )],
                TypeTok::Void,
                vec![TBox::Expr(vec![
                    Token::VarRef(Box::new("println".to_string())),
                    Token::LParen,
                    Token::StructRef(Box::new("this".to_string()), vec!["x".to_string()]),
                    Token::RParen,
                ])]
            ),
            TBox::VarDec(
                Token::VarName(Box::new("me".to_string())),
                None,
                vec![
                    Token::VarRef(Box::new("Point".to_string())),
                    Token::LBrace,
                    Token::VarRef(Box::new("x".to_string())),
                    Token::Colon,
                    Token::IntLit(0),
                    Token::Comma,
                    Token::VarRef(Box::new("y".to_string())),
                    Token::Colon,
                    Token::IntLit(0),
                    Token::RBrace,
                ]
            ),
            TBox::Expr(vec![
                Token::VarRef(Box::new("print_point".to_string())),
                Token::LParen,
                Token::VarRef(Box::new("me".to_string())),
                Token::RParen
            ])
        ]
    )
}
