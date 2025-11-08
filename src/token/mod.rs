use ordered_float::OrderedFloat;
use std::fmt;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    //Lits
    IntLit(i64),
    BoolLit(bool),
    StringLit(Box<String>),
    FloatLit(OrderedFloat<f64>),
    //Infix opps
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    CompoundPlus,
    CompoundMinus,
    CompoundDivide,
    CompoundMultiply,
    PlusPlus,
    MinusMinus,

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
    Else,
    Func,
    Return,
    Break,
    Continue,
    While,
    Struct(Box<String>),
    
    //Names
    VarName(Box<String>),
    VarRef(Box<String>),
    ///Struct, key
    StructRef(Box<String>, Box<String>),
    
    //Syntax
    Semicolon,
    Colon,
    Assign,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    LBrack,
    RBrack,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeTok {
    Int,
    Bool,
    Void,
    Str,
    Any,
    Float,
    ///number is arr dimenstion
    IntArr(u64),
    BoolArr(u64),
    StrArr(u64),
    AnyArr(u64),
    FloatArr(u64),

    Struct(HashMap<String, Box<TypeTok>>),
    StructArr(HashMap<String, Box<TypeTok>>, u64),
    
}

impl Hash for TypeTok {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TypeTok::Int => 1.hash(state),
            TypeTok::Bool => 2.hash(state),
            TypeTok::Void => 3.hash(state),
            TypeTok::Str => 4.hash(state),
            TypeTok::Any => 5.hash(state),
            TypeTok::Float => 6.hash(state),
            TypeTok::IntArr(n) => {7.hash(state); n.hash(state)},
            TypeTok::BoolArr(n) => {8.hash(state); n.hash(state)},
            TypeTok::StrArr(n) => {9.hash(state); n.hash(state)},
            TypeTok::AnyArr(n) => {10.hash(state); n.hash(state)},
            TypeTok::FloatArr(n) => {11.hash(state); n.hash(state)},
            TypeTok::Struct(kv) => {
                12.hash(state); 
                let mut x: Vec<(&String, &Box<TypeTok>)> = kv.iter().collect();
                x.sort_by(|a, b| a.0.cmp(b.0)); // sort by key
                
                x.hash(state);
            },
            TypeTok::StructArr(kv, n) => {
                13.hash(state);
                let mut x: Vec<(&String, &Box<TypeTok>)> = kv.iter().collect();
                x.sort_by(|a, b| a.0.cmp(b.0)); // sort by key
                
                x.hash(state);
                n.hash(state);
            },
        }
    }
}
impl TypeTok {
    pub fn type_str(&self) -> String {
        return match self {
            Self::Bool => "Bool".to_string(),
            Self::Int => "Int".to_string(),
            Self::Void => "Void".to_string(),
            Self::Str => "Str".to_string(),
            Self::Any => "Any".to_string(),
            Self::Float => "Float".to_string(),
            Self::IntArr(_) => "IntArr".to_string(),
            Self::BoolArr(_) => "BoolArr".to_string(),
            Self::StrArr(_) => "StrArr".to_string(),
            Self::FloatArr(_) => "FloatArr".to_string(),
            Self::AnyArr(_) => "AnyArr".to_string(),
            Self::Struct(_) =>"Struct".to_string(),
            Self::StructArr(_, _) => "StructArr".to_string(),
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
            Self::Else => "Else".to_string(),
            Self::LParen => "LParen".to_string(),
            Self::RParen => "RParen".to_string(),
            Self::Func => "Func".to_string(),
            Self::Return => "Return".to_string(),
            Self::Comma => "Comma".to_string(),
            Self::StringLit(_) => "StringLit".to_string(),
            Self::While => "While".to_string(),
            Self::Break => "Break".to_string(),
            Self::Continue => "Continue".to_string(),
            Self::CompoundPlus => "CompoundPlus".to_string(),
            Self::CompoundMinus => "CompoundMinus".to_string(),
            Self::CompoundDivide => "CompoundDivide".to_string(),
            Self::CompoundMultiply => "CompoundMultiply".to_string(),
            Self::PlusPlus => "PlusPlus".to_string(),
            Self::MinusMinus => "MinusMinus".to_string(),
            Self::FloatLit(_) => "FloatLit".to_string(),
            Self::LBrack => "LBrack".to_string(),
            Self::RBrack => "RBrack".to_string(),
            Self::Struct(_) => "Struct".to_string(),
            Self::StructRef(_, _) => "StructRef".to_string()
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
            Self::VarRef(name) => Some(name.clone()),
            __ => None,
        };
    }
    pub fn is_struct_ref(&self) -> bool {
        return match self {
            Self::StructRef(_, _) => true,
            _ => false
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
                Token::Multiply => String::from("MULTIPLY"),
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
                Token::Else => String::from("ELSE"),
                Token::LParen => String::from("LPAREN"),
                Token::RParen => String::from("RPAREN"),
                Token::Func => String::from("FUNC"),
                Token::Return => String::from("RETURN"),
                Token::Comma => String::from("COMMA"),
                Token::StringLit(s) => format!("STRING_LIT({})", s),
                Token::While => String::from("WHILE"),
                Token::Continue => String::from("CONTINUE"),
                Token::Break => String::from("BREAK"),
                Token::CompoundPlus => String::from("COMPOUND_PLUS"),
                Token::CompoundMinus => String::from("COMPOUND_MINUS"),
                Token::CompoundDivide => String::from("COMPOUND_DIVIDE"),
                Token::CompoundMultiply => String::from("COMPOUND_MULTIPLY"),
                Token::PlusPlus => String::from("PLUS_PLUS"),
                Token::MinusMinus => String::from("MINUS_MINUS"),
                Token::FloatLit(f) => format!("Float({})", *f),
                Token::LBrack => String::from("LBRACK"),
                Token::RBrack => String::from("RBRACK"),
                Token::Struct(n) => format!("STRUCT({})", *n),
                Token::StructRef(s, k) => format!("STRUCT_REF STRUCT({}), KEY({})", *s, *k)
            }
        )
    }
}
