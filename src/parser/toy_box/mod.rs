use std::collections::HashMap;
use std::fmt;

use crate::token::{Token, TypeTok};

#[derive(Clone, Debug, PartialEq)]
pub enum TBox {
    Expr(Vec<Token>),
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
    ///Array, idx's, new val
    ArrReassign(Token, Vec<Vec<Token>>, Vec<Token>),
    ///Name, types
    StructInterface(Box<String>, Box<HashMap<String, TypeTok>>),
    ///First token is struct name, then values, then the new value
    StructReassign(Box<String>, Vec<String>, Vec<Token>),
}

impl fmt::Display for TBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TBox::Expr(v) => format!("TBox_expr: {:?}", v),
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
                TBox::ArrReassign(a, i, n) => format!(
                    "TBox_Arr_Reassign Arr({:?}), Index({:?}), NewVal({:?})",
                    a, i, n
                ),
                TBox::StructInterface(n, kv) =>
                    format!("TBox_Struct_Interface Name({}), KV({:?})", *n, *kv),
                TBox::StructReassign(n, f, v) => format!(
                    "TBox_Struct_Reassign  Name({}), Fields({:?}), Value({:?})",
                    *n, f, v
                ),
            }
        )
    }
}
