use std::fmt::{self};

use crate::token::TypeTok;
use ordered_float::OrderedFloat;
use std::collections::HashMap;
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

    StringLit(Box<String>),

    ///Condition, Body
    WhileStmt(Box<Ast>, Vec<Ast>),

    Break,
    Continue,
    FloatLit(OrderedFloat<f64>),

    ///Type, elements
    ArrLit(TypeTok, Vec<Ast>),
    ///Arr, idx
    ArrRef(Box<String>, Vec<Ast>),
    ///Arr, idx, val
    ArrReassign(Box<String>, Vec<Ast>, Box<Ast>),

    ///Name, types
    StructInterface(Box<String>, Box<HashMap<String, TypeTok>>),

    ///Interface name, key, value (types MUST match)
    StructLit(Box<String>, Box<HashMap<String, (Ast, TypeTok)>>),

    ///Struct name (the variable the struct is assigned to NOT the interface), key (key validity and type is checked)
    StructRef(Box<String>, Box<String>),
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
            Ast::StringLit(_) => "StringLit".to_string(),
            Ast::WhileStmt(_, _) => "WhileStmt".to_string(),
            Ast::Continue => "Continue".to_string(),
            Ast::Break => "Break".to_string(),
            Ast::FloatLit(_) => "FloatLit".to_string(),
            Ast::ArrLit(_, _) => "ArrLit".to_string(),
            Ast::ArrRef(_, _) => "ArrRef".to_string(),
            Ast::ArrReassign(_, _, _) => "ArrReassign".to_string(),
            Ast::StructInterface(_, _) => "StructInterface".to_string(),
            Ast::StructLit(_, _) => "StructLit".to_string(),
            Ast::StructRef(_, _) => "StructRef".to_string(),
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
                Ast::StringLit(s) => format!("StringLit Val({})", *s),
                Ast::WhileStmt(cond, body) =>
                    format!("WhileStmt Cond({}), Body({:?})", *cond, body),
                Ast::Break => "Break".to_string(),
                Ast::Continue => "Continue".to_string(),
                Ast::FloatLit(f) => format!("FloatLit({})", *f),
                Ast::ArrLit(t, v) => format!("ArrLit Type({:?}), Val({:?})", t, v),
                Ast::ArrRef(a, i) => format!("ArrRef Arr({:?}), Index({:?})", a, i),
                Ast::ArrReassign(a, i, v) =>
                    format!("ArrReassign Arr({}), Index({:?}), Value({})", *a, i, *v),
                Ast::StructInterface(n, kv) =>
                    format!("StructInterface Name({}), Types({:?})", *n, *kv),
                Ast::StructLit(n, kv) => format!("StructLit Name({}), Types({:?})", *n, *kv),
                Ast::StructRef(n, k) => format!("StructRef Name({}), Key({})", n, k),
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
