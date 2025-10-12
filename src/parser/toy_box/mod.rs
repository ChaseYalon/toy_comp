use std::fmt;

use crate::token::{Token, TypeTok};

#[derive(Clone, Debug, PartialEq)]
pub enum TBox {
    IntExpr(Vec<Token>),
    ///Var name, Var type, Var val
    VarDec(Token, Option<TypeTok>, Vec<Token>),
    #[allow(unused)] //Makes a yellow line go away, it is very much used
    VarRef(Token),
    VarReassign(Token, Vec<Token>)
}

impl fmt::Display for TBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TBox::IntExpr(v) => format!("TBox_INT_EXPR: {:?}", v),
                TBox::VarDec(name, t,  val) => format!("TBox_VAR_DEC: Name({}), Val({:?}), Type({:?})", *name, val, t),
                TBox::VarRef(name) => format!("TBox_VAR_REF: Name({})", *name),
                TBox::VarReassign(var, new_val) => format!("TBox_VAR_REASSIGN Var({}), NewVal({:?})", var, new_val)
            }
        )
    }
}
