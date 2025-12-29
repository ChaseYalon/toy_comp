use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use std::fmt;
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
    Not,

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
    For, //in the context of binding functions to structs
    //Names
    VarName(Box<String>),
    VarRef(Box<String>),
    ///Struct, key
    StructRef(Box<String>, Vec<String>),

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
    Dot,
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

    Struct(BTreeMap<String, Box<TypeTok>>),
    StructArr(BTreeMap<String, Box<TypeTok>>, u64),
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
            TypeTok::IntArr(n) => {
                7.hash(state);
                n.hash(state)
            }
            TypeTok::BoolArr(n) => {
                8.hash(state);
                n.hash(state)
            }
            TypeTok::StrArr(n) => {
                9.hash(state);
                n.hash(state)
            }
            TypeTok::AnyArr(n) => {
                10.hash(state);
                n.hash(state)
            }
            TypeTok::FloatArr(n) => {
                11.hash(state);
                n.hash(state)
            }
            TypeTok::Struct(kv) => {
                12.hash(state);
                let mut x: Vec<(&String, &Box<TypeTok>)> = kv.iter().collect();
                x.sort_by(|a, b| a.0.cmp(b.0)); // sort by key

                x.hash(state);
            }
            TypeTok::StructArr(kv, n) => {
                13.hash(state);
                let mut x: Vec<(&String, &Box<TypeTok>)> = kv.iter().collect();
                x.sort_by(|a, b| a.0.cmp(b.0)); // sort by key

                x.hash(state);
                n.hash(state);
            }
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
            Self::Struct(_) => "Struct".to_string(),
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
            Self::StructRef(_, _) => "StructRef".to_string(),
            Self::Dot => "Dot".to_string(),
            Self::Not => "Not".to_string(),
            Self::For => "For".to_string(),
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
            _ => false,
        };
    }
    pub fn to_string(&self) -> String {
        return format!(
            "{}",
            match self {
                Token::IntLit(n) => format!("{}", n),
                Token::Plus => String::from("+"),
                Token::Minus => String::from("-"),
                Token::Multiply => String::from("*"),
                Token::Divide => String::from("/"),
                Token::Semicolon => String::from(";"),
                Token::Let => String::from("let"),
                Token::Assign => String::from("="),
                Token::VarName(name) => *name.to_owned(),
                Token::VarRef(name) => *name.to_owned(),
                Token::Colon => String::from(":"),
                Token::BoolLit(b) => b.to_string(),
                Token::Type(t) => t.type_str(),
                Token::LessThan => String::from("<"),
                Token::GreaterThan => String::from(">"),
                Token::LessThanEqt => String::from("<="),
                Token::GreaterThanEqt => String::from(">="),
                Token::Equals => String::from("=="),
                Token::NotEquals => String::from("!="),
                Token::Modulo => String::from("%"),
                Token::And => String::from("&&"),
                Token::Or => String::from("||"),
                Token::If => String::from("if"),
                Token::LBrace => String::from("{"),
                Token::RBrace => String::from("}"),
                Token::Else => String::from("else"),
                Token::LParen => String::from("("),
                Token::RParen => String::from(")"),
                Token::Func => String::from("fn"),
                Token::Return => String::from("return"),
                Token::Comma => String::from(","),
                Token::StringLit(s) => *s.to_owned(),
                Token::While => String::from("while"),
                Token::Continue => String::from("continue"),
                Token::Break => String::from("break"),
                Token::CompoundPlus => String::from("+="),
                Token::CompoundMinus => String::from("-="),
                Token::CompoundDivide => String::from("/="),
                Token::CompoundMultiply => String::from("*="),
                Token::PlusPlus => String::from("++"),
                Token::MinusMinus => String::from("--"),
                Token::FloatLit(f) => format!("{}", *f),
                Token::LBrack => String::from("["),
                Token::RBrack => String::from("]"),
                Token::Struct(n) => *n.clone(),
                Token::StructRef(s, k) => {
                    let mut result = (*s).clone();
                    for f in k {
                        result.push_str(f);
                    }
                    result
                }
                Token::Dot => String::from("."),
                Token::Not => String::from("!"),
                Token::For => String::from("for"),
            }
        );
    }
}
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
