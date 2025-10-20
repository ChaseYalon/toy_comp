use crate::token::TypeTok;

use super::{Lexer, Token};
#[test]
fn test_lexer_int_literals() {
    let mut l = Lexer::new();

    //"4"
    let out = l.lex(String::from("4"));
    assert_eq!(out, vec![Token::IntLit(4)]);
}

#[test]
fn test_lexer_infix_ops() {
    let mut l = Lexer::new();

    //"18 - 3 / 6"
    let out = l.lex(String::from("18 - 3 / 6"));
    assert_eq!(
        out,
        vec![
            Token::IntLit(18),
            Token::Minus,
            Token::IntLit(3),
            Token::Divide,
            Token::IntLit(6)
        ]
    );
}

#[test]
fn test_lexer_var_dec() {
    let mut l = Lexer::new();
    let out = l.lex(String::from("let x = 9;"));
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(9),
            Token::Semicolon
        ]
    )
}

#[test]
fn test_lexer_multiple_var_decs() {
    let mut l = Lexer::new();
    let out = l.lex(String::from("let x = 15; let y = 8;"));
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(15),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("y".to_string())),
            Token::Assign,
            Token::IntLit(8),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_var_ref() {
    let mut l = Lexer::new();
    let out = l.lex("let x = 9; x + 3;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(9),
            Token::Semicolon,
            Token::VarRef(Box::new("x".to_string())),
            Token::Plus,
            Token::IntLit(3),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_static_type() {
    let mut l = Lexer::new();
    let out = l.lex("let a: int = 0;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("a".to_string())),
            Token::Colon,
            Token::Type(TypeTok::Int),
            Token::Assign,
            Token::IntLit(0),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_bool_lit() {
    let mut l = Lexer::new();
    let out = l.lex("let b: bool = true;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("b".to_string())),
            Token::Colon,
            Token::Type(TypeTok::Bool),
            Token::Assign,
            Token::BoolLit(true),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_bool_infix() {
    let mut l = Lexer::new();
    let out = l.lex("let b = true; let c = b || false;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("b".to_string())),
            Token::Assign,
            Token::BoolLit(true),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("c".to_string())),
            Token::Assign,
            Token::VarRef(Box::new("b".to_string())),
            Token::Or,
            Token::BoolLit(false),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_misc_infix() {
    let mut l = Lexer::new();
    let out = l.lex("let b = true; let c = b && false; let d = 8; let e = x <= 9;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("b".to_string())),
            Token::Assign,
            Token::BoolLit(true),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("c".to_string())),
            Token::Assign,
            Token::VarRef(Box::new("b".to_string())),
            Token::And,
            Token::BoolLit(false),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("d".to_string())),
            Token::Assign,
            Token::IntLit(8),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("e".to_string())),
            Token::Assign,
            Token::VarRef(Box::new("x".to_string())),
            Token::LessThanEqt,
            Token::IntLit(9),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_misc_infix_2() {
    let mut l = Lexer::new();
    let out = l.lex("let x = 6 >= 0; let b = x != true; let c = 7 == 5;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(6),
            Token::GreaterThanEqt,
            Token::IntLit(0),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("b".to_string())),
            Token::Assign,
            Token::VarRef(Box::new("x".to_string())),
            Token::NotEquals,
            Token::BoolLit(true),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("c".to_string())),
            Token::Assign,
            Token::IntLit(7),
            Token::Equals,
            Token::IntLit(5),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_if_stmt() {
    let mut l = Lexer::new();
    let out = l.lex("if true{}".to_string());
    assert_eq!(
        out,
        vec![
            Token::If,
            Token::BoolLit(true),
            Token::LBrace,
            Token::RBrace,
        ]
    )
}

#[test]
fn test_lexer_nested_if_else() {
    let mut l = Lexer::new();
    let out = l.lex("let x = 4; if x < 10{if true{x = 5}} else {x = 5};".to_string());

    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(4),
            Token::Semicolon,
            Token::If,
            Token::VarRef(Box::new("x".to_string())),
            Token::LessThan,
            Token::IntLit(10),
            Token::LBrace,
            Token::If,
            Token::BoolLit(true),
            Token::LBrace,
            Token::VarRef(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(5),
            Token::RBrace,
            Token::RBrace,
            Token::Else,
            Token::LBrace,
            Token::VarRef(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(5),
            Token::RBrace,
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_nested_parens() {
    let mut l = Lexer::new();
    let out = l.lex("let x = (5 * (3 + 4)) / 7;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::LParen,
            Token::IntLit(5),
            Token::Multiply,
            Token::LParen,
            Token::IntLit(3),
            Token::Plus,
            Token::IntLit(4),
            Token::RParen,
            Token::RParen,
            Token::Divide,
            Token::IntLit(7),
            Token::Semicolon,
        ]
    )
}
#[test]
fn test_lexer_func() {
    let mut l = Lexer::new();
    let out =
        l.lex("fn add(a: int, b: int): int { return a + b; }; let x = add(2, 3);".to_string());

    assert_eq!(
        out,
        vec![
            Token::Func,
            Token::VarName(Box::new("add".to_string())),
            Token::LParen,
            Token::VarRef(Box::new("a".to_string())),
            Token::Colon,
            Token::Type(TypeTok::Int),
            Token::Comma,
            Token::VarRef(Box::new("b".to_string())),
            Token::Colon,
            Token::Type(TypeTok::Int),
            Token::RParen,
            Token::Colon,
            Token::Type(TypeTok::Int),
            Token::LBrace,
            Token::Return,
            Token::VarRef(Box::new("a".to_string())),
            Token::Plus,
            Token::VarRef(Box::new("b".to_string())),
            Token::Semicolon,
            Token::RBrace,
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::VarRef(Box::new("add".to_string())),
            Token::LParen,
            Token::IntLit(2),
            Token::Comma,
            Token::IntLit(3),
            Token::RParen,
            Token::Semicolon
        ]
    )
}

#[test]
fn test_lexer_string_lit() {
    let mut l = Lexer::new();
    let out = l.lex(r#"let x = "hello";"#.to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::StringLit(Box::new("hello".to_string())),
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_call_builtin() {
    let mut l = Lexer::new();
    let out = l.lex(r#"println("hello world");"#.to_string());
    assert_eq!(
        out,
        vec![
            Token::VarRef(Box::new("println".to_string())),
            Token::LParen,
            Token::StringLit(Box::new("hello world".to_string())),
            Token::RParen,
            Token::Semicolon
        ]
    )
}
#[test]
fn test_lexer_str_concat() {
    let mut l = Lexer::new();
    let out = l.lex(r#"let x = "foo" + "bar""#.to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::StringLit(Box::new("foo".to_string())),
            Token::Plus,
            Token::StringLit(Box::new("bar".to_string()))
        ]
    )
}

#[test]
fn test_str_var_concat() {
    let mut l = Lexer::new();
    let out = l.lex(r#"let x = "foo"; let y = "bar"; let z = x + y;"#.to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::StringLit(Box::new("foo".to_string())),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("y".to_string())),
            Token::Assign,
            Token::StringLit(Box::new("bar".to_string())),
            Token::Semicolon,
            Token::Let,
            Token::VarName(Box::new("z".to_string())),
            Token::Assign,
            Token::VarRef(Box::new("x".to_string())),
            Token::Plus,
            Token::VarRef(Box::new("y".to_string())),
            Token::Semicolon,
        ]
    )
}
#[test]
fn test_lexer_print() {
    let mut l = Lexer::new();
    let out = l.lex("print(4);".to_string());
    assert_eq!(
        out,
        vec![
            Token::VarRef(Box::new("print".to_string())),
            Token::LParen,
            Token::IntLit(4),
            Token::RParen,
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_fib() {
    let mut l = Lexer::new();
    let out = l.lex(
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
        "#
        .to_string(),
    );
    assert_eq!(
        out,
        vec![
            Token::Func,
            Token::VarName(Box::new("fib".to_string())),
            Token::LParen,
            Token::VarRef(Box::new("n".to_string())),
            Token::Colon,
            Token::Type(TypeTok::Int),
            Token::RParen,
            Token::Colon,
            Token::Type(TypeTok::Int),
            Token::LBrace,
            Token::If,
            Token::VarRef(Box::new("n".to_string())),
            Token::Equals,
            Token::IntLit(0),
            Token::LBrace,
            Token::Return,
            Token::IntLit(0),
            Token::Semicolon,
            Token::RBrace,
            Token::If,
            Token::VarRef(Box::new("n".to_string())),
            Token::Equals,
            Token::IntLit(1),
            Token::LBrace,
            Token::Return,
            Token::IntLit(1),
            Token::Semicolon,
            Token::RBrace,
            Token::Return,
            Token::VarRef(Box::new("fib".to_string())),
            Token::LParen,
            Token::VarRef(Box::new("n".to_string())),
            Token::Minus,
            Token::IntLit(1),
            Token::RParen,
            Token::Plus,
            Token::VarRef(Box::new("fib".to_string())),
            Token::LParen,
            Token::VarRef(Box::new("n".to_string())),
            Token::Minus,
            Token::IntLit(2),
            Token::RParen,
            Token::Semicolon,
            Token::RBrace,
            Token::VarRef(Box::new("println".to_string())),
            Token::LParen,
            Token::VarRef(Box::new("fib".to_string())),
            Token::LParen,
            Token::IntLit(5),
            Token::RParen,
            Token::RParen,
            Token::Semicolon
        ]
    )
}
#[test]
fn test_lexer_compound_ops() {
    let mut l = Lexer::new();
    let out = l.lex("let x = 5; x += 2; x -= 1; x *= 3; x /= 2; x++; x--;".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(5),
            Token::Semicolon,
            Token::VarRef(Box::new("x".to_string())),
            Token::CompoundPlus,
            Token::IntLit(2),
            Token::Semicolon,
            Token::VarRef(Box::new("x".to_string())),
            Token::CompoundMinus,
            Token::IntLit(1),
            Token::Semicolon,
            Token::VarRef(Box::new("x".to_string())),
            Token::CompoundMultiply,
            Token::IntLit(3),
            Token::Semicolon,
            Token::VarRef(Box::new("x".to_string())),
            Token::CompoundDivide,
            Token::IntLit(2),
            Token::Semicolon,
            Token::VarRef(Box::new("x".to_string())),
            Token::PlusPlus,
            Token::Semicolon,
            Token::VarRef(Box::new("x".to_string())),
            Token::MinusMinus,
            Token::Semicolon,
        ]
    )
}

#[test]
fn test_lexer_while_loop() {
    let mut l = Lexer::new();
    let out =
        l.lex("let x = 0; while x < 10 {if x == 0{continue;} if x == 7 {break;} x++;}".to_string());
    assert_eq!(
        out,
        vec![
            Token::Let,
            Token::VarName(Box::new("x".to_string())),
            Token::Assign,
            Token::IntLit(0),
            Token::Semicolon,
            Token::While,
            Token::VarRef(Box::new("x".to_string())),
            Token::LessThan,
            Token::IntLit(10),
            Token::LBrace,
            Token::If,
            Token::VarRef(Box::new("x".to_string())),
            Token::Equals,
            Token::IntLit(0),
            Token::LBrace,
            Token::Continue,
            Token::Semicolon,
            Token::RBrace,
            Token::If,
            Token::VarRef(Box::new("x".to_string())),
            Token::Equals,
            Token::IntLit(7),
            Token::LBrace,
            Token::Break,
            Token::Semicolon,
            Token::RBrace,
            Token::VarRef(Box::new("x".to_string())),
            Token::PlusPlus,
            Token::Semicolon,
            Token::RBrace,
        ]
    )
}
