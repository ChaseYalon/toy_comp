use crate::{lexer::Lexer, parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer}, token::TypeTok};
use crate::parser::ast::InfixOp;
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
fn test_ast_gen_int_literal(){
    setup_ast!("64", ast);
    assert_eq!(ast, vec![Ast::IntLit(64)])
}

#[test]
fn test_ast_gen_infix_exprs(){
    setup_ast!("18 - 3", ast);
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
    setup_ast!("18 - 3 * 5", ast);
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

#[test]
fn test_ast_gen_var_dec(){
    setup_ast!("let x = 9;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()), 
                TypeTok::Int,
                Box::new(Ast::IntLit(9)),
            )
        ]
    )
}

#[test]
fn test_ast_gen_var_reassign(){
    setup_ast!("let x = 9; x = 5;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9)),
            ),
            Ast::VarReassign(
                Box::new("x".to_string()),
                Box::new(
                    Ast::IntLit(5),
                )
            )
        ]
    )
}

#[test]
fn test_ast_gen_static_type() {
    setup_ast!("let x: int = 9;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9))
            )
        ]
    )
}
#[test]
fn test_ast_gen_bool_lit(){
    setup_ast!("let x: bool = false;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::BoolLit(false))
            )
        ]
    )
}