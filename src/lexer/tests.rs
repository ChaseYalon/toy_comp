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
