use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    IntLit(i64),
    Plus,
    Minus,
    Multiply,
    Divide,
}
impl Token {
    pub fn tok_type(&self) -> &str {
        return match self {
            Self::IntLit(_) => "IntLit",
            Self::Plus => "Plus",
            Self::Minus => "Minus",
            Self::Multiply => "Multiply",
            Self::Divide => "Divide",
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
            }
        )
    }
}
