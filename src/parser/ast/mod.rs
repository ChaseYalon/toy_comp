use std::fmt::{self};

use crate::{errors::Span, token::{QualifiedExternType, TypeTok}};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ast {
    IntLit(i64, Span),
    BoolLit(bool, Span),
    ///lhs, rhs, op, raw text
    InfixExpr(Box<Ast>, Box<Ast>, InfixOp, Span),
    ///Used for Parens, raw text
    EmptyExpr(Box<Ast>, Span),

    ///Variable name, type, value, raw text
    VarDec(Box<String>, TypeTok, Box<Ast>, Span),
    ///var name, raw text
    VarRef(Box<String>, Span),

    ///Condition, body, alt, raw text
    IfStmt(Box<Ast>, Vec<Ast>, Option<Vec<Ast>>, Span),

    ///Name, type, raw text
    FuncParam(Box<String>, TypeTok, Span),

    ///Name, Params, ReturnType, Body, raw text
    FuncDec(Box<String>, Vec<Ast>, TypeTok, Vec<Ast>, Span),

    ///Name, Params, ReturnType, raw text
    ///Params will ALWAYS be ExternFuncParam
    ExternFuncDec(Box<String>, Vec<Ast>, TypeTok, Span),
    ///name type, span
    ExternFuncParam(String, QualifiedExternType, Span),
    ///Name, params as exprs, raw text
    FuncCall(Box<String>, Vec<Ast>, Span),

    ///Val, raw text
    Return(Box<Ast>, Span),
    ///String value, raw text
    StringLit(Box<String>, Span),

    ///Condition, Body, raw text
    WhileStmt(Box<Ast>, Vec<Ast>, Span),

    Break(Span),
    Continue(Span),
    ///Float value
    FloatLit(OrderedFloat<f64>, Span),

    ///Type, elements, raw text
    ArrLit(TypeTok, Vec<Ast>, Span),

    ///Name, types, raw text
    StructInterface(Box<String>, Box<BTreeMap<String, TypeTok>>, Span),

    ///Interface name, key, value (types MUST match), raw text
    StructLit(Box<String>, Box<BTreeMap<String, (Ast, TypeTok)>>, Span),

    ///Target, Index, raw text, used for "[]" operations
    IndexAccess(Box<Ast>, Box<Ast>, Span),
    ///Target, Member, raw text, used for "." operations
    MemberAccess(Box<Ast>, String, Span),
    ///LHS, RHS, raw text
    Assignment(Box<Ast>, Box<Ast>, Span),
    ///find the inverse of a node, must be a boolean expression
    Not(Box<Ast>, Span),
    ///Path to the module being imported from, source code
    ImportStmt(String, Span),

    ///Params, ReturnType, Body, raw text
    LambdaDec(Vec<Ast>, TypeTok, Vec<Ast>, Span),
    ///Callable expression, Args, raw text
    AnonFuncCall(Box<Ast>, Vec<Ast>, Span),
}
impl Ast {
    pub fn node_type(&self) -> String {
        return match self {
            Ast::IntLit(_, _) => "IntLit".to_string(),
            Ast::InfixExpr(_, _, _, _) => "InfixExpr".to_string(),
            Ast::VarDec(_, _, _, _) => "VarDec".to_string(),
            Ast::VarRef(_, _) => "VarRef".to_string(),
            Ast::BoolLit(_, _) => "BoolLit".to_string(),
            Ast::IfStmt(_, _, _, _) => "IfStmt".to_string(),
            Ast::EmptyExpr(_, _) => "EmptyExpr".to_string(),
            Ast::FuncParam(_, _, _) => "FuncParam".to_string(),
            Ast::FuncDec(_, _, _, _, _) => "FuncDec".to_string(),
            Ast::ExternFuncDec(_, _, _, _) => "ExternFuncDec".to_string(),
            Ast::FuncCall(_, _, _) => "FuncCall".to_string(),
            Ast::Return(_, _) => "Return".to_string(),
            Ast::StringLit(_, _) => "StringLit".to_string(),
            Ast::WhileStmt(_, _, _) => "WhileStmt".to_string(),
            Ast::Continue(_) => "Continue".to_string(),
            Ast::Break(_) => "Break".to_string(),
            Ast::FloatLit(_, _) => "FloatLit".to_string(),
            Ast::ArrLit(_, _, _) => "ArrLit".to_string(),
            Ast::StructInterface(_, _, _) => "StructInterface".to_string(),
            Ast::StructLit(_, _, _) => "StructLit".to_string(),
            Ast::IndexAccess(_, _, _) => "IndexAccess".to_string(),
            Ast::MemberAccess(_, _, _) => "MemberAccess".to_string(),
            Ast::Assignment(_, _, _) => "Assignment".to_string(),
            Ast::Not(_, _) => "Not".to_string(),
            Ast::ImportStmt(_, _) => "ImportStmt".to_string(),
            Ast::ExternFuncParam(_, _, _) => "ExternFuncParam".to_string(),
            Ast::LambdaDec(_, _, _, _) => "LambdaDec".to_string(),
            Ast::AnonFuncCall(_, _, _) => "AnonFuncCall".to_string(),
        };
    }

    pub fn span(&self) -> Span {
        match self {
            Ast::InfixExpr(_, _, _, s) => s.clone(),
            Ast::IntLit(_, s) => s.clone(),
            Ast::VarDec(_, _, _, s) => s.clone(),
            Ast::VarRef(_, s) => s.clone(),
            Ast::BoolLit(_, s) => s.clone(),
            Ast::IfStmt(_, _, _, s) => s.clone(),
            Ast::EmptyExpr(_, s) => s.clone(),
            Ast::FuncParam(_, _, s) => s.clone(),
            Ast::FuncDec(_, _, _, _, s) => s.clone(),
            Ast::ExternFuncDec(_, _, _, s) => s.clone(),
            Ast::FuncCall(_, _, s) => s.clone(),
            Ast::Return(_, s) => s.clone(),
            Ast::StringLit(_, s) => s.clone(),
            Ast::WhileStmt(_, _, s) => s.clone(),
            Ast::Break(s) => s.clone(),
            Ast::Continue(s) => s.clone(),
            Ast::FloatLit(_, s) => s.clone(),
            Ast::ArrLit(_, _, s) => s.clone(),
            Ast::StructInterface(_, _, s) => s.clone(),
            Ast::StructLit(_, _, s) => s.clone(),
            Ast::IndexAccess(_, _, s) => s.clone(),
            Ast::MemberAccess(_, _, s) => s.clone(),
            Ast::Assignment(_, _, s) => s.clone(),
            Ast::Not(_, s) => s.clone(),
            Ast::ImportStmt(_, s) => s.clone(),
            Ast::ExternFuncParam(_, _, s) => s.clone(),
            Ast::LambdaDec(_, _, _, s) => s.clone(),
            Ast::AnonFuncCall(_, _, s) => s.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
                Ast::InfixExpr(a, b, c, s) => format!(
                    "INFIX_EXPR left({}), Right({}), Opp({}), Literal({})",
                    *a, *b, c, s
                ),
                Ast::IntLit(i, _) => format!("INT({:.2})", i),
                Ast::VarDec(name, var_type, value, s) => format!(
                    "Name({}), Value({}), Type({:?}), Literal({})",
                    *name, value, var_type, s
                ),
                Ast::VarRef(var, s) => format!("Var({}), Literal({})", *var, s),
                Ast::BoolLit(b, _) => format!("BoolLit({})", b),
                Ast::IfStmt(cond, body, alt, s) => format!(
                    "IfStmt Cond({}), Body({:?}), Alt({:?}), Literal({})",
                    cond, body, alt, s
                ),
                Ast::EmptyExpr(child, s) => format!("EmptyExpr({}), Literal({})", child, s),
                Ast::FuncParam(name, type_tok, s) => format!(
                    "FuncParam Name({}), Type({:?}), Literal({})",
                    *name, type_tok, s
                ),
                Ast::FuncDec(name, params, return_type, body, s) => format!(
                    "FuncDec Name({}), Params({:?}), ReturnType({:?}), Body({:?}), Literal({})",
                    *name, params, return_type, body, s
                ),
                Ast::ExternFuncDec(name, params, return_type, s) => format!(
                    "ExternFuncDec Name({}), Params({:?}), ReturnType({:?}), Literal({})",
                    *name, params, return_type, s
                ),
                Ast::FuncCall(name, params, s) => format!(
                    "FuncCall, Name({}), Params({:?}), Literal({})",
                    *name, params, s
                ),
                Ast::Return(val, s) => format!("Return Val({}), Literal({})", *val, s),
                Ast::StringLit(st, s) => format!("StringLit Val({}), Literal({})", *st, s),
                Ast::WhileStmt(cond, body, s) => format!(
                    "WhileStmt Cond({}), Body({:?}), Literal({})",
                    *cond, body, s
                ),
                Ast::Break(_) => "Break".to_string(),
                Ast::Continue(_) => "Continue".to_string(),
                Ast::FloatLit(fl, _) => format!("FloatLit({})", *fl),
                Ast::ArrLit(t, v, s) =>
                    format!("ArrLit Type({:?}), Val({:?}), Literal({})", t, v, s),
                Ast::StructInterface(n, kv, s) => format!(
                    "StructInterface Name({}), Types({:?}), Literal({})",
                    *n, *kv, s
                ),
                Ast::StructLit(n, kv, s) =>
                    format!("StructLit Name({}), Types({:?}), Literal({})", *n, *kv, s),
                Ast::Not(n, _) => format!("Not({})", *n),
                Ast::IndexAccess(t, i, s) =>
                    format!("IndexAccess Target({}), Index({}), Literal({})", *t, *i, s),
                Ast::MemberAccess(t, m, s) =>
                    format!("MemberAccess Target({}), Member({}), Literal({})", *t, m, s),
                Ast::Assignment(l, r, s) =>
                    format!("Assignment LHS({}), RHS({}), Literal({})", *l, *r, s),
                Ast::ImportStmt(path, s) => format!("ImportStmt Path({}), Literal({})", path, s),
                Ast::ExternFuncParam(n, t, s) => format!("ExternFuncParam Name({}), Type({:?}), Literal({})", n, t, s),
                Ast::LambdaDec(params, ret, body, s) => format!(
                    "LambdaDec Params({:?}), ReturnType({:?}), Body({:?}), Literal({})",
                    params, ret, body, s
                ),
                Ast::AnonFuncCall(callable, args, s) => format!(
                    "AnonFuncCall Callable({}), Args({:?}), Literal({})",
                    *callable, args, s
                ),
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
