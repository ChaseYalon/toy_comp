use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    IntLit(i64),
    InfixExpr(Box<Ast>, Box<Ast>, InfixOp),
}
impl Ast{
    pub fn node_type(&self) -> &str{
        return match self{
            Ast::IntLit(_) => "IntLit",
            Ast::InfixExpr(_, _, _) => "InfixExpr",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum InfixOp {
    Plus,
    Minus,
    Divide,
    Multiply,
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
            }
        )
    }
}
