use std::fmt;

use crate::token::{Token, TypeTok};

#[derive(Clone, Debug, PartialEq)]
pub enum TBox {
    Expr(Vec<Token>),
    ///Var name, Var type, Var val
    VarDec(Token, Option<TypeTok>, Vec<Token>),
    #[allow(unused)] //Makes a yellow line go away, it is very much used
    VarRef(Token),
    VarReassign(Token, Vec<Token>),
    ///Cond, body, Optional else
    IfStmt(Vec<Token>, Vec<TBox>, Option<Vec<TBox>>),
    ///Name, type
    FuncParam(Token, TypeTok),
    ///Name, Params, Return Type, Body
    FuncDec(Token, Vec<TBox>, TypeTok, Vec<TBox>),
    ///Contains value to return
    Return(Box<TBox>),
    ///Condition, body
    While(Vec<Token>, Vec<TBox>),
    Break,
    Continue,
}

impl fmt::Display for TBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TBox::Expr(v) => format!("TBox_expr: {:?}", v),
                TBox::VarDec(name, t, val) => format!(
                    "TBox_VAR_DEC: Name({}), Val({:?}), Type({:?})",
                    *name, val, t
                ),
                TBox::VarRef(name) => format!("TBox_VAR_REF: Name({})", *name),
                TBox::VarReassign(var, new_val) =>
                    format!("TBox_VAR_REASSIGN Var({}), NewVal({:?})", var, new_val),
                TBox::IfStmt(cond, body, alt) => format!(
                    "TBox_If_Stmt Cond({:?}), Body({:?}), Alt({:?})",
                    cond, body, alt
                ),
                TBox::FuncParam(name, t) =>
                    format!("TBox_Func_Param Name({}), Type({:?})", name, t),
                TBox::FuncDec(name, params, return_type, body) => format!(
                    "TBox_Func_Dec Name({}), Params({:?}), ReturnType({:?}, Body({:?})",
                    name, params, return_type, body
                ),
                TBox::Return(val) => format!("TBox_Return Val({:?})", val),
                TBox::While(cond, body) => format!("TBox_While Cond({:?}), Body({:?})", cond, body),
                TBox::Break => "TBox_break".to_string(),
                TBox::Continue => "TBox_continue".to_string(),
            }
        )
    }
}
