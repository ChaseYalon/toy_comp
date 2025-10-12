use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    IntLit(i64),
    Plus,
    Minus,
    Multiply,
    Divide,
    Semicolon,
    Let,
    Assign,
    VarName(Box<String>),
    VarRef(Box<String>),
    Colon,
    BoolLit(bool),
    Type(TypeTok)
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeTok {
    Int,
    Bool,
}
impl TypeTok{
    pub fn type_str(&self) -> String{
        return match self {
            Self::Bool => "Bool".to_string(),
            Self::Int => "Int".to_string()
        }
    }
}
impl Token {
    pub fn tok_type(&self) -> String {
        return match self {
            Self::IntLit(_) => "IntLit".to_string(),
            Self::Plus => "Plus".to_string(),
            Self::Minus => "Minus".to_string(),
            Self::Multiply => "Multiply".to_string(),
            Self::Divide => "Divide".to_string(),
            Self::Semicolon => "Semicolon".to_string(),
            Self::Let => "Let".to_string(),
            Self::Assign => "Assign".to_string(),
            Self::VarName(_) => "VarName".to_string(),
            Self::VarRef(_) => "VarRef".to_string(),
            Self::Colon => "Colon".to_string(),
            Self::BoolLit(_) => "BoolLit".to_string(),
            Self::Type(_) => "Type".to_string(),
        };
    }
    ///Is used to get value out of an int literal
    pub fn get_val(&self) -> Option<i64> {
        return match self {
            Self::IntLit(i) => Some(i.clone()),
            _ => None,
        };
    }
    pub fn get_var_name(&self) -> Option<Box<String>>{
        return match self {
            Self::VarName(name) => Some(name.clone()),
            __ => None
        }
    }
}
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Token::IntLit(n) => format!("INT({})", n),
                Token::Plus => String::from("PLUS"),
                Token::Minus => String::from("MINUS"),
                Token::Multiply => String::from("DIVIDE"),
                Token::Divide => String::from("DIVIDE"),
                Token::Semicolon => String::from("SEMICOLON"),
                Token::Let => String::from("LET"),
                Token::Assign => String::from("ASSIGN"),
                Token::VarName(name) => format!("VarName({})", *name),
                Token::VarRef(name) => format!("VarRef({})", *name),
                Token::Colon => String::from("COLON"),
                Token::BoolLit(b) => format!("BoolLit({})", b),
                Token::Type(t) => format!("Type({:?})", t)
            }
        )
    }
}
