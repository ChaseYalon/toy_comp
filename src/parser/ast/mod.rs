use std::fmt::{self};

use crate::token::TypeTok;

#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    IntLit(i64),
    BoolLit(bool),
    InfixExpr(Box<Ast>, Box<Ast>, InfixOp),
    ///Used for Parens
    EmptyExpr(Box<Ast>),

    ///Variable name, type, value
    VarDec(Box<String>, TypeTok, Box<Ast>),
    VarRef(Box<String>),
    ///Variable name and expression to assign it to
    VarReassign(Box<String>, Box<Ast>),

    ///Condition, body, alt
    IfStmt(Box<Ast>, Vec<Ast>, Option<Vec<Ast>>),

    ///Name, type
    FuncParam(Box<String>, TypeTok),

    ///Name, Params, ReturnType, Body,
    FuncDec(Box<String>, Vec<Ast>, TypeTok, Vec<Ast>),

    ///Name, params as exprs
    FuncCall(Box<String>, Vec<Ast>),

    ///Val
    Return(Box<Ast>),
}
impl Ast {
    pub fn node_type(&self) -> String {
        return match self {
            Ast::IntLit(_) => "IntLit".to_string(),
            Ast::InfixExpr(_, _, _) => "InfixExpr".to_string(),
            Ast::VarDec(_, _, _) => "VarDec".to_string(),
            Ast::VarRef(_) => "VarRef".to_string(),
            Ast::VarReassign(_, _) => "VarReassign".to_string(),
            Ast::BoolLit(_) => "BoolLit".to_string(),
            Ast::IfStmt(_, _, _) => "IfStmt".to_string(),
            Ast::EmptyExpr(_) => "EmptyExpr".to_string(),
            Ast::FuncParam(_, _) => "FuncParam".to_string(),
            Ast::FuncDec(_, _, _, _) => "FuncDec".to_string(),
            Ast::FuncCall(_, _) => "FuncCall".to_string(),
            Ast::Return(_) => "Return".to_string(),
        };
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum InfixOp {
    Plus,
    Minus,
    Divide,
    Multiply,
    LessThan,
    LessThanEqt,
    GreaterThan,
    GreaterThanEqt,
    NotEquals,
    Equals,
    Modulo,
    And,
    Or,
}
impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Ast::InfixExpr(a, b, c) =>
                    format!("INFIX_EXPR left({}), Right({}), Opp({})", *a, *b, c),
                Ast::IntLit(i) => format!("INT({:.2}", i),
                Ast::VarDec(name, var_type, value) =>
                    format!("Name({}), Value({}), Type({:?})", *name, value, var_type),
                Ast::VarRef(var) => format!("Var({})", *var),
                Ast::VarReassign(var, val) => format!("Var({}) = Val({:?})", *var, *val),
                Ast::BoolLit(b) => format!("BoolLit({})", b),
                Ast::IfStmt(cond, body, alt) =>
                    format!("IfStmt Cond({}), Body({:?}), Alt({:?})", cond, body, alt),
                Ast::EmptyExpr(child) => format!("EmptyExpr({})", child),
                Ast::FuncParam(name, type_tok) =>
                    format!("FuncParam Name({}), Type({:?})", *name, type_tok),
                Ast::FuncDec(name, params, return_type, body) => format!(
                    "FuncDec Name({}), Params({:?}), ReturnType({:?}), Body({:?})",
                    *name, params, return_type, body
                ),
                Ast::FuncCall(name, params) =>
                    format!("FuncCall, Name({}), Params({:?})", *name, params),
                Ast::Return(val) => format!("Return Val({})", *val),
            }
        )
    }
}

impl fmt::Display for InfixOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InfixOp::Plus => "PLUS",
                InfixOp::Minus => "MINUS",
                InfixOp::Multiply => "MULTIPLY",
                InfixOp::Divide => "Divide",
                InfixOp::LessThan => "LESS_THAN",
                InfixOp::LessThanEqt => "LESS_THAN_EQT",
                InfixOp::GreaterThan => "GREATER_THAN",
                InfixOp::GreaterThanEqt => "GREATER_THAN_EQT",
                InfixOp::NotEquals => "NOT_EQUALS",
                InfixOp::Equals => "EQUALS",
                InfixOp::Modulo => "MODULO",
                InfixOp::And => "AND",
                InfixOp::Or => "OR",
            }
        )
    }
}
