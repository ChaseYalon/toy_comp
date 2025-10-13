use crate::debug;
use crate::token::{Token, TypeTok};

#[derive(Debug)]
pub struct Lexer {
    chars: Vec<char>,
    cp: usize, //Char pointer
    num_buf: Vec<char>,
    str_buf: Vec<char>,
    toks: Vec<Token>,
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
        };
    }
    pub fn peek(&self, i: usize) -> char {
        if self.chars.len() < self.cp + i {
            //Not sure what to do here
            return '\0';
        }
        debug!(i, self.cp, &self.chars);
        return self.chars[self.cp + i];
    }
    fn lex_keyword(&mut self, word: &str, tok: Token) -> bool {
        for (i, c) in word.char_indices() {
            if self.peek(i) != c {
                return false;
            }
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
            debug!(c);
            debug!(self.cp);
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
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

            if c.is_ascii_digit() {
                debug!("In ascii print");
                self.num_buf.push(c);
                self.eat();
                continue;
            }
            if c == '+' {
                self.flush();
                self.toks.push(Token::Plus);
                self.eat();
                continue;
            }
            if c == '-' {
                self.flush();
                self.toks.push(Token::Minus);
                self.eat();
                continue;
            }
            if c == '*' {
                self.flush();
                self.toks.push(Token::Multiply);
                self.eat();
                continue;
            }
            if c == '/' {
                self.flush();
                self.toks.push(Token::Divide);
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
            if c == '!' {
                self.flush();
                if self.peek(1) == '=' {
                    self.toks.push(Token::NotEquals);
                    self.cp += 2;
                    continue;
                }
                todo!("Chase, you have to implement the not operator in the lexer");
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
            if c.is_ascii() {
                self.flush_int();
                self.str_buf.push(c);
                self.eat();
                continue;
            }
            panic!("[ERROR] Unexpected token, got {}", c);
        }
        debug!(self.toks.clone());

        //Catch any trailing its
        self.flush_int();
        let to_ret = self.toks.clone();
        self.clean_up();
        return to_ret;
    }
    fn eat(&mut self) {
        self.cp += 1;
    }
    fn flush(&mut self) {
        self.flush_int();
        self.flush_str();
    }
    fn flush_int(&mut self) {
        if self.num_buf.len() == 0 {
            return;
        }
        let proto_output: String = self.num_buf.clone().into_iter().collect();
        let output: i64 = proto_output.parse().unwrap();
        self.num_buf = Vec::new();
        self.toks.push(Token::IntLit(output));
    }
    fn flush_str(&mut self) {
        if self.str_buf.len() == 0 {
            return;
        }
        if self.toks.last().unwrap().tok_type() == "Let" {
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
