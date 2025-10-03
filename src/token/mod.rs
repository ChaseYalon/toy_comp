use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    IntLit(i64),
    Plus,
    Minus,
    Multiply,
    Divide,
    Semicolon,
}
impl Token {
    pub fn tok_type(&self) -> &str {
        return match self {
            Self::IntLit(_) => "IntLit",
            Self::Plus => "Plus",
            Self::Minus => "Minus",
            Self::Multiply => "Multiply",
            Self::Divide => "Divide",
            Self::Semicolon => "Semicolon",
        };
    }
    ///Is used to get value out of an int literal
    pub fn get_val(&self) -> Option<i64> {
        return match self {
            Self::IntLit(i) => Some(i.clone()),
            _ => None,
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
            }
        )
    }
}
