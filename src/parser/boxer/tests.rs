use super::{Boxer, TBox, Token};
use crate::lexer::Lexer;

#[test]
fn test_boxer_int_literal() {
    let input = String::from("4");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let toks = l.lex(input);
    let boxes = b.box_toks(toks);

    assert_eq!(boxes, vec![TBox::IntExpr(vec![Token::IntLit(4)]),])
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
        vec![TBox::IntExpr(vec![
            Token::IntLit(8),
            Token::Minus,
            Token::IntLit(3),
            Token::Multiply,
            Token::IntLit(5),
        ]),]
    )
}
