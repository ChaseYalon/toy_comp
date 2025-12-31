use crate::debug;
use crate::errors::{ToyError, ToyErrorType};
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
        if self.cp + i >= self.chars.len() {
            //Not sure what to do hereP
            return '\0';
        }
        debug!(targets: ["lexer", "lexer_verbose"], i, self.cp, &self.chars);
        return self.chars[self.cp + i];
    }
    /// Get a snippet of text around the current position for error reporting
    fn get_error_context(&self) -> String {
        let start = self.cp.saturating_sub(10);
        let end = (self.cp + 10).min(self.chars.len());
        self.chars[start..end].iter().collect()
    }
    fn lex_keyword(&mut self, word: &str, tok: Token) -> bool {
        for (i, c) in word.char_indices() {
            if self.peek(i) != c {
                return false;
            }
        }
        //next char must flush buffer
        let next_char = self.peek(word.len());
        // don't match keywords that are part of longer identifiers
        // previous char must not be alphanumeric or '_'
        let prev_char = if self.cp == 0 {
            '\0'
        } else {
            self.chars[self.cp - 1]
        };
        if prev_char.is_alphanumeric() || prev_char == '_' {
            return false;
        }
        // lparen is not alphanumeric but otherwise int will match print()
        if (next_char.is_alphanumeric() || next_char == '_') || next_char == '(' {
            return false;
        }

        self.cp += word.len();
        self.toks.push(tok);
        return true;
    }
    fn lex_arr_def(&mut self, word: &str, base_type: TypeTok) -> Result<bool, ToyError> {
        for (i, c) in word.char_indices() {
            if self.peek(i) != c {
                return Ok(false);
            }
        }

        let mut arr_count = 0;
        let mut i = self.cp + word.len();
        let len = self.chars.len();

        while i + 1 < len && self.chars[i] == '[' && self.chars[i + 1] == ']' {
            arr_count += 1;
            i += 2;
        }

        if arr_count == 0 {
            return Ok(false);
        }

        let arr_type = match base_type {
            TypeTok::Int => TypeTok::IntArr(arr_count),
            TypeTok::Bool => TypeTok::BoolArr(arr_count),
            TypeTok::Str => TypeTok::StrArr(arr_count),
            TypeTok::Float => TypeTok::FloatArr(arr_count),
            TypeTok::Any => TypeTok::AnyArr(arr_count),
            _ => {
                return Err(ToyError::new(
                    ToyErrorType::ArrayTypeInvalid,
                    Some(self.get_error_context()),
                ));
            }
        };

        self.toks.push(Token::Type(arr_type));
        self.cp = i;
        return Ok(true);
    }

    pub fn lex(&mut self, input: String) -> Result<Vec<Token>, ToyError> {
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
            //lex arrs before regular type to avoid int[] becoming (int) empty array
            if self.lex_arr_def("int", TypeTok::Int)? {
                continue;
            }
            if self.lex_arr_def("bool", TypeTok::Bool)? {
                continue;
            }
            if self.lex_arr_def("str", TypeTok::Str)? {
                continue;
            }
            if self.lex_arr_def("float", TypeTok::Float)? {
                continue;
            }
            if self.lex_arr_def("any", TypeTok::Any)? {
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
            if self.lex_keyword("void", Token::Type(TypeTok::Void)) {
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
            if self.lex_keyword("any", Token::Type(TypeTok::Any)) {
                continue;
            }
            if self.lex_keyword("struct", Token::Struct(Box::new("".to_string()))) {
                continue;
            }
            if self.lex_keyword("for", Token::For) {
                continue;
            }
            if self.lex_keyword("extern", Token::Extern) {
                continue;
            }
            if self.lex_keyword("import", Token::Import) {
                continue;
            }
                if c.is_ascii_digit() || (c == '.' && self.num_buf.len() > 0) {
                debug!(targets: ["lexer_verbose"], "In ascii print");
                self.num_buf.push(c);
                self.eat();
                continue;
            }
            if c == '.' {
                self.flush();
                if self.toks.len() == 0 {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedFieldName,
                        Some(self.get_error_context()),
                    ));
                }
                match self.toks.last().unwrap() {
                    Token::VarName(_) | Token::VarRef(_) | Token::RBrack | Token::RParen => {}
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::MalformedFieldName,
                            Some(self.get_error_context()),
                        ));
                    }
                }
                let len = self.chars.len();
                loop {
                    self.cp += 1;
                    let mut field_name = String::new();

                    while self.cp < self.chars.len()
                        && (self.chars[self.cp].is_alphanumeric() || self.chars[self.cp] == '_')
                    {
                        field_name.push(self.chars[self.cp]);
                        self.cp += 1;
                    }

                    if field_name.is_empty() {
                        return Err(ToyError::new(
                            ToyErrorType::MalformedFieldName,
                            Some(self.get_error_context()),
                        ));
                    }

                    self.toks.push(Token::Dot);
                    self.toks.push(Token::VarRef(Box::new(field_name)));

                    if self.cp < len && self.chars[self.cp] == '.' {
                        continue;
                    } else {
                        break;
                    }
                }

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
                if self.peek(1).is_ascii_digit() {
                    let is_unary = self.toks.is_empty() || {
                        match self.toks.last().unwrap() {
                            Token::Assign
                            | Token::LParen
                            | Token::LBrack
                            | Token::LBrace
                            | Token::Comma
                            | Token::Colon
                            | Token::Plus
                            | Token::Minus
                            | Token::Multiply
                            | Token::Divide
                            | Token::Modulo
                            | Token::Equals
                            | Token::NotEquals
                            | Token::LessThan
                            | Token::GreaterThan
                            | Token::LessThanEqt
                            | Token::GreaterThanEqt
                            | Token::And
                            | Token::Or
                            | Token::Not
                            | Token::Return
                            | Token::If
                            | Token::While
                            | Token::Semicolon
                            | Token::CompoundPlus
                            | Token::CompoundMinus
                            | Token::CompoundMultiply
                            | Token::CompoundDivide => true,
                            _ => false,
                        }
                    };
                    if is_unary {
                        self.num_buf.push('-');
                        self.eat();
                        continue;
                    }
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
                self.toks.push(Token::Not);
                self.eat();
                continue;
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
            if c == '[' {
                self.flush();
                self.toks.push(Token::LBrack);
                self.eat();
                continue;
            }
            if c == ']' {
                self.flush();
                self.toks.push(Token::RBrack);
                self.eat();
                continue;
            }
            if c.is_ascii() {
                self.flush_num();
                self.str_buf.push(c);
                self.eat();
                continue;
            }

            return Err(ToyError::new(
                ToyErrorType::UnknownCharacter(c),
                Some(self.get_error_context()),
            ));
        }
        debug!(targets: ["lexer_verbose"], self.toks.clone());

        //Catch any trailing its
        self.flush_num();
        let to_ret = self.toks.clone();
        self.clean_up();
        return Ok(to_ret);
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
            let output: f64 = proto_output.parse().unwrap();
            self.num_buf = Vec::new();
            self.toks.push(Token::FloatLit(OrderedFloat(output)));
            return;
        }
        let output: i64 = proto_output.parse().unwrap();

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

        if !(self.toks.len() == 0)
            && self.toks.last().unwrap() == &Token::Struct(Box::new("".to_string()))
        {
            let proto_output: String = self.str_buf.clone().into_iter().collect();
            let len = self.toks.clone().len();
            self.toks[len - 1] = Token::Struct(Box::new(proto_output));
            self.str_buf = Vec::new();
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
