use super::{Boxer, TBox, Token};
use crate::{lexer::Lexer, token::TypeTok};

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
                )]
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
                    )]
                )
            ]
        )]
    )
}
