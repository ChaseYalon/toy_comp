use std::collections::BTreeMap;
use std::fmt;

use crate::{errors::Span, token::{SpannedToken, TypeTok}};

#[derive(Clone, Debug, PartialEq)]
pub enum TBox {
    ///tokens in the expr, original code for that expression
    Expr(Vec<SpannedToken>, Span),
    ///Var name, Var type, Var val, source code for that expression
    VarDec(SpannedToken, Option<TypeTok>, Vec<SpannedToken>, Span),
    ///represents a reassignment of the tokens LHS to the value RHS
    Assign(Vec<SpannedToken>, Vec<SpannedToken>, Span),
    ///Cond, body, Optional cond, body pairs for else if, Optional else, original code
    IfStmt(
        Vec<SpannedToken>,
        Vec<TBox>,
        Option<Vec<(Vec<SpannedToken>, Vec<TBox>)>>,
        Option<Vec<TBox>>,
        Span,
    ),
    ///Name, type, source code
    FuncParam(SpannedToken, TypeTok, Span),
    ///Name, Params, Return Type, Body, source code, isExport - defaults to false
    FuncDec(SpannedToken, Vec<TBox>, TypeTok, Vec<TBox>, Span, bool),
    ///Contains value to return, source code
    Return(Box<TBox>, Span),
    ///Condition, body, Source code
    While(Vec<SpannedToken>, Vec<TBox>, Span),
    Break(Span),
    Continue(Span),
    ///Name, types, Source code
    StructInterface(Box<String>, Box<BTreeMap<String, TypeTok>>, Span),
    ///used for extern function declarations, those functions are called like any other
    ///Name, Params, Return Type, source code
    ExternFuncDec(SpannedToken, Vec<TBox>, TypeTok, Span),
    ///name of the module being imported, source_code
    ImportStmt(String, Span),
    ///Interfaces just contain the TypeTok of the interface, then the source code
    Interface(TypeTok, Span),
}
impl TBox {
    ///will return the types of a func param, if it is given on a func_dec node, will return nothing otherwise
    pub fn get_func_param_types(&self) -> Vec<TypeTok> {
        match self {
            TBox::FuncDec(_, p, _, _, _, _) => {
                let mut v: Vec<TypeTok> = vec![];
                for param in p {
                    match param {
                        TBox::FuncParam(_, t, _) => v.push(t.clone()),
                        _ => unreachable!(),
                    }
                }
                return v;
            }
            _ => {}
        }
        return vec![];
    }
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
                TBox::Assign(lhs, rhs, s) => {
                    format!("TBox_Assign LHS({:?}), RHS({:?}), Literal({})", lhs, rhs, s)
                }
                TBox::IfStmt(cond, body, alt, else_body, s) => format!(
                    "TBox_If_Stmt Cond({:?}), Body({:?}), Alt({:?}), Else({:?}), Literal({})",
                    cond, body, alt, else_body, s
                ),
                TBox::FuncParam(name, t, s) => format!(
                    "TBox_Func_Param Name({}), Type({:?}), Literal({})",
                    name, t, s
                ),
                TBox::FuncDec(name, params, return_type, body, s, is_export) => format!(
                    "TBox_Func_Dec Name({}), Params({:?}), ReturnType({:?}, Body({:?}), Literal({}), IsExport({})",
                    name, params, return_type, body, s, is_export
                ),
                TBox::Return(val, s) => format!("TBox_Return Val({:?}), Literal({})", val, s),
                TBox::While(cond, body, s) => format!(
                    "TBox_While Cond({:?}), Body({:?}), Literal({})",
                    cond, body, s
                ),
                TBox::Break(_) => "TBox_break".to_string(),
                TBox::Continue(_) => "TBox_continue".to_string(),
                TBox::StructInterface(n, kv, s) => format!(
                    "TBox_Struct_Interface Name({}), KV({:?}), Literal({})",
                    *n, *kv, s
                ),
                TBox::ExternFuncDec(name, params, return_type, s) => format!(
                    "TBox_Extern_Func_Dec Name({}), Params({:?}), ReturnType({:?}), Literal({})",
                    name, params, return_type, s
                ),
                TBox::ImportStmt(name, s) =>
                    format!("TBox_Import_Stmt Name({}), Literal({})", name, s),
                TBox::Interface(ty, s) => format!("TBox_Interface Type({:#?}), Literal({})", ty, s),
            }
        )
    }
}
