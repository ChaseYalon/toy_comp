use std::collections::BTreeMap;
use std::fmt;

use crate::token::{Token, TypeTok};

#[derive(Clone, Debug, PartialEq)]
pub enum TBox {
    ///tokens in the expr, original code for that expression
    Expr(Vec<Token>, String),
    ///Var name, Var type, Var val, source code for that expression
    VarDec(Token, Option<TypeTok>, Vec<Token>, String),
    #[allow(unused)] //Makes a yellow line go away, it is very much used
    VarRef(Token),
    ///var token, new val, original code
    VarReassign(Token, Vec<Token>, String),
    ///Cond, body, Optional else, original code
    IfStmt(Vec<Token>, Vec<TBox>, Option<Vec<TBox>>, String),
    ///Name, type, source code
    FuncParam(Token, TypeTok, String),
    ///Name, Params, Return Type, Body, source code
    FuncDec(Token, Vec<TBox>, TypeTok, Vec<TBox>, String),
    ///Contains value to return, source code
    Return(Box<TBox>, String),
    ///Condition, body, Source code
    While(Vec<Token>, Vec<TBox>, String),
    Break,
    Continue,
    ///Array, idx's, new val, Source code
    ArrReassign(Token, Vec<Vec<Token>>, Vec<Token>, String),
    ///Name, types, Source code
    StructInterface(Box<String>, Box<BTreeMap<String, TypeTok>>, String),
    ///First token is struct name, then values, then the new value, Source code
    StructReassign(Box<String>, Vec<String>, Vec<Token>, String),
}

impl fmt::Display for TBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TBox::Expr(v, s) => format!("TBox_expr: {:?}, Literal({})", v, s),
                TBox::VarDec(name, t, val, s) => format!(
                    "TBox_VAR_DEC: Name({}), Val({:?}), Type({:?}) Literal({})",
                    *name, val, t, s
                ),
                TBox::VarRef(name) => format!("TBox_VAR_REF: Name({})", *name),
                TBox::VarReassign(var, new_val, s) => format!(
                    "TBox_VAR_REASSIGN Var({}), NewVal({:?}) Literal({})",
                    var, new_val, s
                ),
                TBox::IfStmt(cond, body, alt, s) => format!(
                    "TBox_If_Stmt Cond({:?}), Body({:?}), Alt({:?}), Literal({})",
                    cond, body, alt, s
                ),
                TBox::FuncParam(name, t, s) => format!(
                    "TBox_Func_Param Name({}), Type({:?}), Literal({})",
                    name, t, s
                ),
                TBox::FuncDec(name, params, return_type, body, s) => format!(
                    "TBox_Func_Dec Name({}), Params({:?}), ReturnType({:?}, Body({:?}), Literal({})",
                    name, params, return_type, body, s
                ),
                TBox::Return(val, s) => format!("TBox_Return Val({:?}), Literal({})", val, s),
                TBox::While(cond, body, s) => format!(
                    "TBox_While Cond({:?}), Body({:?}), Literal({})",
                    cond, body, s
                ),
                TBox::Break => "TBox_break".to_string(),
                TBox::Continue => "TBox_continue".to_string(),
                TBox::ArrReassign(a, i, n, s) => format!(
                    "TBox_Arr_Reassign Arr({:?}), Index({:?}), NewVal({:?}), Literal({})",
                    a, i, n, s
                ),
                TBox::StructInterface(n, kv, s) => format!(
                    "TBox_Struct_Interface Name({}), KV({:?}), Literal({})",
                    *n, *kv, s
                ),
                TBox::StructReassign(n, f, v, s) => format!(
                    "TBox_Struct_Reassign  Name({}), Fields({:?}), Value({:?}), Literal({})",
                    *n, f, v, s
                ),
            }
        )
    }
}
