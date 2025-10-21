use crate::debug;
use crate::token::{Token, TypeTok};
use ordered_float::OrderedFloat;

#[derive(Debug)]
pub struct Lexer {
    chars: Vec<char>,
    cp: usize, //Char pointer
    num_buf: Vec<char>,
    str_buf: Vec<char>,
    toks: Vec<Token>,
    in_str_lit: bool,
}
impl Lexer {
    pub fn new() -> Lexer {
        let c_vec: Vec<char> = Vec::new();
        let n_vec: Vec<char> = Vec::new();
        let t_vec: Vec<Token> = Vec::new();
        let s_vec: Vec<char> = Vec::new();
        return Lexer {
            chars: c_vec,
            cp: 0usize,
            num_buf: n_vec,
            str_buf: s_vec,
            toks: t_vec,
            in_str_lit: false,
        };
    }
    pub fn peek(&self, i: usize) -> char {
        if self.chars.len() < self.cp + i {
            //Not sure what to do here
            return '\0';
        }
        debug!(targets: ["lexer", "lexer_verbose"], i, self.cp, &self.chars);
        return self.chars[self.cp + i];
    }
    fn lex_keyword(&mut self, word: &str, tok: Token) -> bool {
        for (i, c) in word.char_indices() {
            if self.peek(i) != c {
                return false;
            }
        }
        //next char must flush buffer
        let next_char = self.peek(word.len());
        //lparen is not alphanumeric but otherwise int will match print()
        if (next_char.is_alphanumeric() || next_char == '_') || next_char == '(' {
            return false;
        }

        self.cp += word.len();
        self.toks.push(tok);
        return true;
    }
    pub fn lex(&mut self, input: String) -> Vec<Token> {
        self.chars = input.chars().collect();
        self.cp = 0;

        while self.cp < self.chars.len() {
            let c = self.chars[self.cp];
            if self.in_str_lit {
                self.eat();
                if c == '"' {
                    self.flush();
                    self.in_str_lit = false;
                    continue;
                }
                self.str_buf.push(c);
                continue;
            }
            debug!(targets: ["lexer_verbose"], c);
            debug!(targets: ["lexer_verbose"], self.cp);
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r') && !self.in_str_lit {
                self.eat();
                continue;
            }
            if self.lex_keyword("let", Token::Let) {
                continue;
            }
            if self.lex_keyword("int", Token::Type(TypeTok::Int)) {
                continue;
            }
            if self.lex_keyword("bool", Token::Type(TypeTok::Bool)) {
                continue;
            }
            if self.lex_keyword("true", Token::BoolLit(true)) {
                continue;
            }
            if self.lex_keyword("false", Token::BoolLit(false)) {
                continue;
            }
            if self.lex_keyword("if", Token::If) {
                continue;
            }
            if self.lex_keyword("else", Token::Else) {
                continue;
            }
            if self.lex_keyword("fn", Token::Func) {
                continue;
            }
            if self.lex_keyword("return", Token::Return) {
                continue;
            }
            if self.lex_keyword("str", Token::Type(TypeTok::Str)) {
                continue;
            }
            if self.lex_keyword("while", Token::While) {
                continue;
            }
            if self.lex_keyword("break", Token::Break) {
                continue;
            }
            if self.lex_keyword("continue", Token::Continue) {
                continue;
            }
            if self.lex_keyword("float", Token::Type(TypeTok::Float)) {
                continue;
            }

            if c.is_ascii_digit() || c == '.' {
                debug!(targets: ["lexer_verbose"], "In ascii print");
                self.num_buf.push(c);
                self.eat();
                continue;
            }
            if c == '+' {
                if self.peek(1) == '=' {
                    self.flush();
                    self.toks.push(Token::CompoundPlus);
                    self.cp += 2;
                    continue;
                }
                if self.peek(1) == '+' {
                    self.flush();
                    self.toks.push(Token::PlusPlus);
                    self.cp += 2;
                    continue;
                }
                self.flush();
                self.toks.push(Token::Plus);
                self.eat();
                continue;
            }
            if c == '-' {
                if self.peek(1) == '=' {
                    self.flush();
                    self.toks.push(Token::CompoundMinus);
                    self.cp += 2;
                    continue;
                }
                if self.peek(1) == '-' {
                    self.flush();
                    self.toks.push(Token::MinusMinus);
                    self.cp += 2;
                    continue;
                }
                self.flush();
                self.toks.push(Token::Minus);
                self.eat();
                continue;
            }
            if c == '*' {
                if self.peek(1) == '=' {
                    self.flush();
                    self.toks.push(Token::CompoundMultiply);
                    self.cp += 2;
                    continue;
                }
                self.flush();
                self.toks.push(Token::Multiply);
                self.eat();
                continue;
            }
            if c == '/' {
                if self.peek(1) == '=' {
                    self.flush();
                    self.toks.push(Token::CompoundDivide);
                    self.cp += 2;
                    continue;
                }
                self.flush();
                self.toks.push(Token::Divide);
                self.eat();
                continue;
            }
            if c == '(' {
                self.flush();
                self.toks.push(Token::LParen);
                self.eat();
                continue;
            }
            if c == ')' {
                self.flush();
                self.toks.push(Token::RParen);
                self.eat();
                continue;
            }
            if c == '%' {
                self.flush();
                self.toks.push(Token::Modulo);
                self.eat();
                continue;
            }
            if c == '&' && self.peek(1) == '&' {
                self.flush();
                self.toks.push(Token::And);
                self.cp += 2;
                continue;
            }
            if c == '|' && self.peek(1) == '|' {
                self.flush();
                self.toks.push(Token::Or);
                self.cp += 2;
                continue;
            }
            if c == '<' {
                self.flush();
                if self.peek(1) == '=' {
                    self.toks.push(Token::LessThanEqt);
                    self.cp += 2;
                    continue;
                }
                self.toks.push(Token::LessThan);
                self.eat();
                continue;
            }
            if c == '>' {
                self.flush();
                if self.peek(1) == '=' {
                    self.toks.push(Token::GreaterThanEqt);
                    self.cp += 2;
                    continue;
                }
                self.toks.push(Token::GreaterThan);
                self.eat();
                continue;
            }
            if c == '=' {
                self.flush();
                if self.peek(1) == '=' {
                    self.toks.push(Token::Equals);
                    self.cp += 2;
                    continue;
                }
                self.toks.push(Token::Assign);
                self.eat();
                continue;
            }
            if c == '"' {
                if self.in_str_lit {
                    self.flush_str();
                    self.in_str_lit = false;
                } else {
                    self.flush();
                    self.in_str_lit = true;
                }
                self.eat();
                continue;
            }
            if c == '!' {
                self.flush();
                if self.peek(1) == '=' {
                    self.toks.push(Token::NotEquals);
                    self.cp += 2;
                    continue;
                }
                todo!("Chase, you have to implement the not operator in the lexer");
            }
            if c == '{' {
                self.flush();
                self.toks.push(Token::LBrace);
                self.eat();
                continue;
            }
            if c == '}' {
                self.flush();
                self.toks.push(Token::RBrace);
                self.eat();
                continue;
            }
            if c == ';' {
                self.flush();
                self.toks.push(Token::Semicolon);
                self.eat();
                continue;
            }
            if c == ':' {
                self.flush();
                self.toks.push(Token::Colon);
                self.eat();
                continue;
            }
            if c == ',' {
                self.flush();
                self.toks.push(Token::Comma);
                self.eat();
                continue;
            }
            if c.is_ascii() {
                self.flush_num();
                self.str_buf.push(c);
                self.eat();
                continue;
            }

            panic!("[ERROR] Unexpected token, got {}", c);
        }
        debug!(targets: ["lexer_verbose"], self.toks.clone());

        //Catch any trailing its
        self.flush_num();
        let to_ret = self.toks.clone();
        self.clean_up();
        return to_ret;
    }
    fn eat(&mut self) {
        self.cp += 1;
    }
    fn flush(&mut self) {
        self.flush_num();
        self.flush_str();
    }
    fn flush_num(&mut self) {
        if self.num_buf.len() == 0 {
            return;
        }
        let proto_output: String = self.num_buf.clone().into_iter().collect();
        if proto_output.contains('.') {
            println!("{:?}", proto_output);
            let output: f64 = proto_output.parse().unwrap();
            self.num_buf = Vec::new();
            self.toks.push(Token::FloatLit(OrderedFloat(output)));
            return;
        }
        let output:i64  = proto_output.parse().unwrap();

        self.num_buf = Vec::new();
        self.toks.push(Token::IntLit(output));
    }
    fn flush_str(&mut self) {
        if self.in_str_lit {
            let proto_output: String = self.str_buf.clone().into_iter().collect();
            self.toks.push(Token::StringLit(Box::new(proto_output)));
            self.str_buf = Vec::new();
            return;
        }

        if self.str_buf.len() == 0 {
            return;
        }

        if self.toks.len() == 0 {
            let proto_output: String = self.str_buf.clone().into_iter().collect();
            self.toks.push(Token::VarRef(Box::new(proto_output)));
            self.str_buf = Vec::new();
            return;
        }
        if self.toks.last().unwrap().tok_type() == "Let"
            || self.toks.last().unwrap().tok_type() == "Func"
        {
            let proto_output: String = self.str_buf.clone().into_iter().collect();
            self.toks.push(Token::VarName(Box::new(proto_output)));
            self.str_buf = Vec::new();
        } else {
            let proto_output: String = self.str_buf.clone().into_iter().collect();
            self.toks.push(Token::VarRef(Box::new(proto_output)));
            self.str_buf = Vec::new();
        }
    }

    fn clean_up(&mut self) {
        self.chars = Vec::new();
        self.cp = 0;
        self.num_buf = Vec::new();
        self.str_buf = Vec::new();
        self.toks = Vec::new();
    }
}

//Loads and executes tests
#[cfg(test)]
mod tests;
