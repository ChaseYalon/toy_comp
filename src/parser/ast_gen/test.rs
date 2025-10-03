use crate::{lexer::Lexer, parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer}};
use crate::parser::ast::InfixOp;


#[test]
fn test_ast_gen_int_literal(){
    let input = String::from("64");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let mut a = AstGenerator::new();

    let toks = l.lex(input);
    let boxes = b.box_toks(toks);
    let ast = a.generate(boxes);

    assert_eq!(ast, vec![Ast::IntLit(64)])
}

#[test]
fn test_ast_gen_infix_exprs(){
    let input = String::from("18 - 3");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let mut a = AstGenerator::new();

    let toks = l.lex(input);
    let boxes = b.box_toks(toks);
    let ast = a.generate(boxes);

    assert_eq!(ast, 
        vec![
            Ast::InfixExpr(
                Box::new(Ast::IntLit(18)),
                Box::new(Ast::IntLit(3)), 
                InfixOp::Minus
            )
        ]
    )
}

#[test]
fn test_ast_gen_order_ops(){
    let input = String::from("18 - 3 * 5");
    let mut l = Lexer::new();
    let mut b = Boxer::new();
    let mut a = AstGenerator::new();

    let toks = l.lex(input);
    let boxes = b.box_toks(toks);
    let ast = a.generate(boxes);

    assert_eq!(ast, 
        vec![
            Ast::InfixExpr(
                Box::new(Ast::IntLit(18)),
                Box::new(
                    Ast::InfixExpr(
                        Box::new(Ast::IntLit(3)),
                        Box::new(Ast::IntLit(5)), 
                        InfixOp::Multiply
                    )
                ),
                InfixOp::Minus
            )
        ]
    )
}