use std::fmt;


#[derive(Debug, Clone)]
pub enum Token {
    IntLit(i32),
    InfixExpr(Box<Token>, Box<Token>, Operator),
}
impl Token{
    pub fn tok_type(&self)->&str{
        return match self{
            Self::IntLit(_) => "IntLit",
            Self::InfixExpr(_, _, _) => "InfixExpr"
        }
    }
    ///This is meant to be for int literals, DONT CALL IT ON SOMETHING YOU ARE NOT SURE IS AN INT LITERAL
    pub fn get_val(&self) -> Option<i32>{
        return match &self{
            Token::IntLit(n) => Some(*n),
            _ => None
        }
    }
    ///This returns the left and the right value of an infix expression
    pub fn get_sides(&self) -> Option<(&Box<Token>, &Box<Token>)>{
        return match &self{
            Token::InfixExpr(l,r ,_) => Some((l,r)),
            _ => None
        }
    }
    ///This returns the operator of an infix expression
    pub fn get_op(&self) -> Option<&Operator>{
        return match &self{
            Token::InfixExpr(_, _, o) => Some(o),
            _ => None,
        }
    }
    
}
impl fmt::Display for Token{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Token::IntLit(n) => format!("INT({})", n),
            Token::InfixExpr(l, r, o) => format!("INFIX_EXPR LEFT({}) RIGHT({}) OPERATOR ({})", l, r, o),
        })
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}
impl fmt::Display for Operator{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{
        write!(f, "{}", match self{
            Operator::Add => "ADD",
            Operator::Sub => "SUB",
            Operator::Mul => "MUL",
            Operator::Div =>"DIV",
        })
    }
}