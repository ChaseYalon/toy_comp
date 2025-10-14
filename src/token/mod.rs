use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    //Lits
    IntLit(i64),
    BoolLit(bool),

    //Infix opps
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,

    //Boolean infix opps
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanEqt,
    GreaterThanEqt,
    And,
    Or,

    //Keywords
    Let,
    Type(TypeTok),
    If,

    //Names
    VarName(Box<String>),
    VarRef(Box<String>),

    //Syntax
    Semicolon,
    Colon,
    Assign,
    LBrace,
    RBrace,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeTok {
    Int,
    Bool,
}
impl TypeTok {
    pub fn type_str(&self) -> String {
        return match self {
            Self::Bool => "Bool".to_string(),
            Self::Int => "Int".to_string(),
        };
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
            Self::LessThan => "LessThan".to_string(),
            Self::GreaterThan => "GreaterThan".to_string(),
            Self::LessThanEqt => "LessThanEqt".to_string(),
            Self::GreaterThanEqt => "GreaterThanEqt".to_string(),
            Self::Equals => "Equals".to_string(),
            Self::NotEquals => "NotEquals".to_string(),
            Self::Modulo => "Modulo".to_string(),
            Self::And => "And".to_string(),
            Self::Or => "Or".to_string(),
            Self::If => "If".to_string(),
            Self::LBrace => "LBrace".to_string(),
            Self::RBrace => "RBrace".to_string(),
        };
    }
    ///Is used to get value out of an int literal
    pub fn get_val(&self) -> Option<i64> {
        return match self {
            Self::IntLit(i) => Some(i.clone()),
            _ => None,
        };
    }
    pub fn get_var_name(&self) -> Option<Box<String>> {
        return match self {
            Self::VarName(name) => Some(name.clone()),
            __ => None,
        };
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
                Token::Type(t) => format!("Type({:?})", t),
                Token::LessThan => String::from("LESS_THAN"),
                Token::GreaterThan => String::from("GREATER_THAN"),
                Token::LessThanEqt => String::from("LESS_THAN_EQT"),
                Token::GreaterThanEqt => String::from("GREATER_THAN_EQT"),
                Token::Equals => String::from("EQUALS"),
                Token::NotEquals => String::from("NOT_EQUALS"),
                Token::Modulo => String::from("MODULO"),
                Token::And => String::from("AND"),
                Token::Or => String::from("OR"),
                Token::If => String::from("IF"),
                Token::LBrace => String::from("LBRACE"),
                Token::RBrace => String::from("RBRACE"),
            }
        )
    }
}
