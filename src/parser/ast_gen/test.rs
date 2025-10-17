use crate::parser::ast::InfixOp;
use crate::{
    lexer::Lexer,
    parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer},
    token::TypeTok,
};
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
fn test_ast_gen_int_literal() {
    setup_ast!("64", ast);
    assert_eq!(ast, vec![Ast::IntLit(64)])
}

#[test]
fn test_ast_gen_infix_exprs() {
    setup_ast!("18 - 3", ast);
    assert_eq!(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(18)),
            Box::new(Ast::IntLit(3)),
            InfixOp::Minus
        )]
    )
}

#[test]
fn test_ast_gen_order_ops() {
    setup_ast!("18 - 3 * 5", ast);
    assert_eq!(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(18)),
            Box::new(Ast::InfixExpr(
                Box::new(Ast::IntLit(3)),
                Box::new(Ast::IntLit(5)),
                InfixOp::Multiply
            )),
            InfixOp::Minus
        )]
    )
}

#[test]
fn test_ast_gen_var_dec() {
    setup_ast!("let x = 9;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::IntLit(9)),
        )]
    )
}

#[test]
fn test_ast_gen_var_reassign() {
    setup_ast!("let x = 9; x = 5;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9)),
            ),
            Ast::VarReassign(Box::new("x".to_string()), Box::new(Ast::IntLit(5),))
        ]
    )
}

#[test]
fn test_ast_gen_static_type() {
    setup_ast!("let x: int = 9;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Int,
            Box::new(Ast::IntLit(9))
        )]
    )
}
#[test]
fn test_ast_gen_bool_lit() {
    setup_ast!("let x: bool = false;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Bool,
            Box::new(Ast::BoolLit(false))
        )]
    )
}

#[test]
fn test_ast_gen_bool_infix() {
    setup_ast!(
        "let foo = 8 > 4 || false; let x = 9; let bar = 9 == x;",
        ast
    );

    assert_eq!(
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
                    )),
                    Box::new(Ast::BoolLit(false)),
                    InfixOp::Or,
                ))
            ),
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(9))
            ),
            Ast::VarDec(
                Box::new("bar".to_string()),
                TypeTok::Bool,
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(9)),
                    Box::new(Ast::VarRef(Box::new("x".to_string()))),
                    InfixOp::Equals
                ))
            )
        ]
    )
}

#[test]
fn test_ast_gen_mixed_bool_int() {
    setup_ast!("let x = 4 + 3 < 6;", ast);
    assert_eq!(
        ast,
        vec![Ast::VarDec(
            Box::new("x".to_string()),
            TypeTok::Bool,
            Box::new(Ast::InfixExpr(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::IntLit(4)),
                    Box::new(Ast::IntLit(3)),
                    InfixOp::Plus
                )),
                Box::new(Ast::IntLit(6)),
                InfixOp::LessThan
            ))
        )]
    )
}
#[test]
fn test_asg_gen_modulo() {
    setup_ast!("5 % 3;", ast);
    assert_eq!(
        ast,
        vec![Ast::InfixExpr(
            Box::new(Ast::IntLit(5)),
            Box::new(Ast::IntLit(3)),
            InfixOp::Modulo
        )]
    )
}

#[test]
fn test_ast_gen_return_bool() {
    setup_ast!("let x: bool = true; x || false;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::BoolLit(true)),
            ),
            Ast::InfixExpr(
                Box::new(Ast::VarRef(Box::new("x".to_string()))),
                Box::new(Ast::BoolLit(false)),
                InfixOp::Or
            )
        ]
    )
}

#[test]
fn test_ast_gen_if_stmt() {
    setup_ast!("let x = false; if x || true {x = true;}", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Bool,
                Box::new(Ast::BoolLit(false))
            ),
            Ast::IfStmt(
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new("x".to_string()))),
                    Box::new(Ast::BoolLit(true)),
                    InfixOp::Or,
                )),
                vec![Ast::VarReassign(
                    Box::new("x".to_string()),
                    Box::new(Ast::BoolLit(true))
                )],
                None,
            )
        ]
    )
}

#[test]
fn test_ast_gen_if_stmt_complex() {
    setup_ast!("let x:int = 8; if true {x = 4}; x;", ast);
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(8)),
            ),
            Ast::IfStmt(
                Box::new(Ast::BoolLit(true)),
                vec![Ast::VarReassign(
                    Box::new("x".to_string()),
                    Box::new(Ast::IntLit(4))
                )],
                None
            ),
            Ast::VarRef(Box::new("x".to_string())),
        ]
    )
}

#[test]
fn test_ast_gen_if_else() {
    setup_ast!("if true && false {let x = 7;} else {let x = 8;}", ast);
    assert_eq!(
        ast,
        vec![Ast::IfStmt(
            Box::new(Ast::InfixExpr(
                Box::new(Ast::BoolLit(true)),
                Box::new(Ast::BoolLit(false)),
                InfixOp::And,
            )),
            vec![Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(7))
            )],
            Some(vec![Ast::VarDec(
                Box::new("x".to_string()),
                TypeTok::Int,
                Box::new(Ast::IntLit(8))
            )])
        )]
    )
}

#[test]
fn test_ast_gen_nested_parens() {
    setup_ast!("let x = (5 * (3 + 4)) / 7;", ast);
    //This nesting is a crime against humanity
    assert_eq!(
        ast,
        vec![
            Ast::VarDec(
                Box::new("x".to_string()), 
                TypeTok::Int, 
                Box::new(
                    Ast::InfixExpr(
                        Box::new(
                            Ast::EmptyExpr(
                                Box::new(
                                    Ast::InfixExpr(
                                        Box::new(Ast::IntLit(5)), 
                                        Box::new(
                                            Ast::EmptyExpr(
                                                Box::new(
                                                    Ast::InfixExpr(
                                                        Box::new(Ast::IntLit(3)), 
                                                        Box::new(Ast::IntLit(4)), 
                                                        InfixOp::Plus
                                                    )
                                                )
                                            )
                                        ), 
                                        InfixOp::Multiply
                                    )
                                )
                            )
                        ), 
                        Box::new(Ast::IntLit(7)), 
                        InfixOp::Divide
                    )
                )
            )
        ]
    )
}