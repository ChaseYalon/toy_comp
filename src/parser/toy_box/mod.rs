use std::fmt;

use crate::token::Token;

#[derive(Clone, Debug, PartialEq)]
pub enum TBox {
    IntExpr(Vec<Token>),
}

impl fmt::Display for TBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TBox::IntExpr(v) => format!("TBox_INT_EXPR: {:?}", v),
            }
        )
    }
}
