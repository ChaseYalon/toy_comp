use std::fmt::{self, Debug};

use crate::{
    errors::Span,
    token::{QualifiedExternType, TypeTok},
};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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
                Ast::ExternFuncParam(n, t, s) =>
                    format!("ExternFuncParam Name({}), Type({:?}), Literal({})", n, t, s),
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

impl Debug for InfixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InfixOp::Plus => "Plus",
            InfixOp::Minus => "Minus",
            InfixOp::Multiply => "Multiply",
            InfixOp::Divide => "Divide",
            InfixOp::LessThan => "LessThan",
            InfixOp::LessThanEqt => "LessThanEqt",
            InfixOp::GreaterThan => "GreaterThan",
            InfixOp::GreaterThanEqt => "GreaterThanEqt",
            InfixOp::NotEquals => "NotEquals",
            InfixOp::Equals => "Equals",
            InfixOp::Modulo => "Modulo",
            InfixOp::And => "And",
            InfixOp::Or => "Or",
        };
        f.write_str(s)
    }
}

impl Debug for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let skip_spans = env::var("TOY_PP")
            .map(|v| v.to_uppercase() == "TRUE")
            .unwrap_or(false);

        if skip_spans {
            self.fmt_no_spans(f)
        } else {
            self.fmt_with_spans(f)
        }
    }
}

impl Ast {
    fn fmt_no_spans(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ast::IntLit(i, _) => f.debug_tuple("IntLit").field(i).finish(),
            Ast::BoolLit(b, _) => f.debug_tuple("BoolLit").field(b).finish(),
            Ast::InfixExpr(lhs, rhs, op, _) => f
                .debug_tuple("InfixExpr")
                .field(lhs)
                .field(rhs)
                .field(op)
                .finish(),
            Ast::EmptyExpr(expr, _) => f.debug_tuple("EmptyExpr").field(expr).finish(),
            Ast::VarDec(name, ty, value, _) => f
                .debug_tuple("VarDec")
                .field(name)
                .field(ty)
                .field(value)
                .finish(),
            Ast::VarRef(name, _) => f.debug_tuple("VarRef").field(name).finish(),
            Ast::IfStmt(cond, body, alt, _) => f
                .debug_tuple("IfStmt")
                .field(cond)
                .field(body)
                .field(alt)
                .finish(),
            Ast::FuncParam(name, ty, _) => {
                f.debug_tuple("FuncParam").field(name).field(ty).finish()
            }
            Ast::FuncDec(name, params, return_type, body, _) => f
                .debug_tuple("FuncDec")
                .field(name)
                .field(params)
                .field(return_type)
                .field(body)
                .finish(),
            Ast::ExternFuncDec(name, params, return_type, _) => f
                .debug_tuple("ExternFuncDec")
                .field(name)
                .field(params)
                .field(return_type)
                .finish(),
            Ast::FuncCall(name, params, _) => {
                f.debug_tuple("FuncCall").field(name).field(params).finish()
            }
            Ast::Return(val, _) => f.debug_tuple("Return").field(val).finish(),
            Ast::StringLit(s, _) => f.debug_tuple("StringLit").field(s).finish(),
            Ast::WhileStmt(cond, body, _) => {
                f.debug_tuple("WhileStmt").field(cond).field(body).finish()
            }
            Ast::Break(_) => f.debug_struct("Break").finish(),
            Ast::Continue(_) => f.debug_struct("Continue").finish(),
            Ast::FloatLit(fl, _) => f.debug_tuple("FloatLit").field(fl).finish(),
            Ast::ArrLit(ty, elements, _) => {
                f.debug_tuple("ArrLit").field(ty).field(elements).finish()
            }
            Ast::StructInterface(name, fields, _) => f
                .debug_tuple("StructInterface")
                .field(name)
                .field(fields)
                .finish(),
            Ast::StructLit(name, fields, _) => f
                .debug_tuple("StructLit")
                .field(name)
                .field(fields)
                .finish(),
            Ast::IndexAccess(target, index, _) => f
                .debug_tuple("IndexAccess")
                .field(target)
                .field(index)
                .finish(),
            Ast::MemberAccess(target, member, _) => f
                .debug_tuple("MemberAccess")
                .field(target)
                .field(member)
                .finish(),
            Ast::Assignment(lhs, rhs, _) => {
                f.debug_tuple("Assignment").field(lhs).field(rhs).finish()
            }
            Ast::Not(expr, _) => f.debug_tuple("Not").field(expr).finish(),
            Ast::ImportStmt(path, _) => f.debug_tuple("ImportStmt").field(path).finish(),
            Ast::ExternFuncParam(name, ty, _) => f
                .debug_tuple("ExternFuncParam")
                .field(name)
                .field(ty)
                .finish(),
            Ast::LambdaDec(name, ret_ty, body, _) => f
                .debug_tuple("LambdaDec")
                .field(name)
                .field(ret_ty)
                .field(body)
                .finish(),
            Ast::AnonFuncCall(v, p, _) => f.debug_tuple("AnonFuncCall").field(v).field(p).finish(),
        }
    }

    fn fmt_with_spans(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ast::IntLit(i, s) => f.debug_tuple("IntLit").field(i).field(s).finish(),
            Ast::BoolLit(b, s) => f.debug_tuple("BoolLit").field(b).field(s).finish(),
            Ast::InfixExpr(lhs, rhs, op, s) => f
                .debug_tuple("InfixExpr")
                .field(lhs)
                .field(rhs)
                .field(op)
                .field(s)
                .finish(),
            Ast::EmptyExpr(expr, s) => f.debug_tuple("EmptyExpr").field(expr).field(s).finish(),
            Ast::VarDec(name, ty, value, s) => f
                .debug_tuple("VarDec")
                .field(name)
                .field(ty)
                .field(value)
                .field(s)
                .finish(),
            Ast::VarRef(name, s) => f.debug_tuple("VarRef").field(name).field(s).finish(),
            Ast::IfStmt(cond, body, alt, s) => f
                .debug_tuple("IfStmt")
                .field(cond)
                .field(body)
                .field(alt)
                .field(s)
                .finish(),
            Ast::FuncParam(name, ty, s) => f
                .debug_tuple("FuncParam")
                .field(name)
                .field(ty)
                .field(s)
                .finish(),
            Ast::FuncDec(name, params, return_type, body, s) => f
                .debug_tuple("FuncDec")
                .field(name)
                .field(params)
                .field(return_type)
                .field(body)
                .field(s)
                .finish(),
            Ast::ExternFuncDec(name, params, return_type, s) => f
                .debug_tuple("ExternFuncDec")
                .field(name)
                .field(params)
                .field(return_type)
                .field(s)
                .finish(),
            Ast::FuncCall(name, params, s) => f
                .debug_tuple("FuncCall")
                .field(name)
                .field(params)
                .field(s)
                .finish(),
            Ast::Return(val, s) => f.debug_tuple("Return").field(val).field(s).finish(),
            Ast::StringLit(st, s) => f.debug_tuple("StringLit").field(st).field(s).finish(),
            Ast::WhileStmt(cond, body, s) => f
                .debug_tuple("WhileStmt")
                .field(cond)
                .field(body)
                .field(s)
                .finish(),
            Ast::Break(s) => f.debug_struct("Break").field("span", s).finish(),
            Ast::Continue(s) => f.debug_struct("Continue").field("span", s).finish(),
            Ast::FloatLit(fl, s) => f.debug_tuple("FloatLit").field(fl).field(s).finish(),
            Ast::ArrLit(ty, elements, s) => f
                .debug_tuple("ArrLit")
                .field(ty)
                .field(elements)
                .field(s)
                .finish(),
            Ast::StructInterface(name, fields, s) => f
                .debug_tuple("StructInterface")
                .field(name)
                .field(fields)
                .field(s)
                .finish(),
            Ast::StructLit(name, fields, s) => f
                .debug_tuple("StructLit")
                .field(name)
                .field(fields)
                .field(s)
                .finish(),
            Ast::IndexAccess(target, index, s) => f
                .debug_tuple("IndexAccess")
                .field(target)
                .field(index)
                .field(s)
                .finish(),
            Ast::MemberAccess(target, member, s) => f
                .debug_tuple("MemberAccess")
                .field(target)
                .field(member)
                .field(s)
                .finish(),
            Ast::Assignment(lhs, rhs, s) => f
                .debug_tuple("Assignment")
                .field(lhs)
                .field(rhs)
                .field(s)
                .finish(),
            Ast::Not(expr, s) => f.debug_tuple("Not").field(expr).field(s).finish(),
            Ast::ImportStmt(path, s) => f.debug_tuple("ImportStmt").field(path).field(s).finish(),
            Ast::ExternFuncParam(name, ty, s) => f
                .debug_tuple("ExternFuncParam")
                .field(name)
                .field(ty)
                .field(s)
                .finish(),
            Ast::LambdaDec(name, ret_ty, body, s) => f
                .debug_tuple("LambdaDec")
                .field(name)
                .field(ret_ty)
                .field(body)
                .field(s)
                .finish(),
            Ast::AnonFuncCall(v, p, s) => f
                .debug_tuple("AnonFuncCall")
                .field(v)
                .field(p)
                .field(s)
                .finish(),
        }
    }
}
