use crate::debug;
use crate::driver::Driver;
use crate::errors::{Span, ToyError, ToyErrorType};
use crate::token::{SpannedToken, Token, TypeTok};
use ordered_float::OrderedFloat;

#[derive(Debug)]
pub struct Lexer {
    /// The source text decoded into individual chars
    source_chars: Vec<char>,
    /// Current position in `source_chars`
    cursor: usize,
    /// Accumulator for numeric literals (int / float)
    number_buffer: Vec<char>,
    /// Accumulator for identifier / string literal chars
    string_buffer: Vec<char>,
    /// Tokens produced so far (parallel to `token_start_bytes`)
    pending_tokens: Vec<Token>,
    /// Whether we are currently inside a string literal
    in_string_literal: bool,
    /// Maps char index → byte offset in the original UTF-8 source
    char_byte_offsets: Vec<usize>,
    /// Byte offset where each pending token started (parallel to `pending_tokens`)
    token_start_bytes: Vec<u64>,
    /// Char index where the current number/string buffer started accumulating
    buffer_start_cursor: usize,
}
impl Lexer {
    pub fn new() -> Lexer {
        return Lexer {
            source_chars: Vec::new(),
            cursor: 0,
            number_buffer: Vec::new(),
            string_buffer: Vec::new(),
            pending_tokens: Vec::new(),
            in_string_literal: false,
            char_byte_offsets: Vec::new(),
            token_start_bytes: Vec::new(),
            buffer_start_cursor: 0,
        };
    }
    pub fn peek(&self, offset: usize) -> char {
        if self.cursor + offset >= self.source_chars.len() {
            return '\0';
        }
        debug!(targets: ["lexer", "lexer_verbose"], offset, self.cursor, &self.source_chars);
        return self.source_chars[self.cursor + offset];
    }
    /// Push a token, recording `start_cursor` (char index) as its span start.
    fn push_tok(&mut self, tok: Token, start_cursor: usize) {
        let start_byte = self
            .char_byte_offsets
            .get(start_cursor)
            .copied()
            .unwrap_or(0) as u64;
        self.token_start_bytes.push(start_byte);
        self.pending_tokens.push(tok);
    }
    /// Build a Span pointing at the current character for error reporting.
    fn get_error_span(&self) -> Span {
        let file_path = Driver::get_current_file_path().unwrap_or_else(|| "<unknown>".to_string());
        if self.char_byte_offsets.is_empty() || self.cursor >= self.char_byte_offsets.len() {
            return Span::null_span_with_msg(&format!("near end of {}", file_path));
        }
        let byte = self.char_byte_offsets[self.cursor] as i64;
        Span::new(&file_path, byte, byte)
    }
    fn lex_keyword(&mut self, word: &str, tok: Token) -> bool {
        let start_cursor = self.cursor;
        for (i, c) in word.char_indices() {
            if self.peek(i) != c {
                return false;
            }
        }
        let next_char = self.peek(word.len());
        // don't match keywords that are part of longer identifiers
        let prev_char = if self.cursor == 0 {
            '\0'
        } else {
            self.source_chars[self.cursor - 1]
        };
        if prev_char.is_alphanumeric() || prev_char == '_' {
            return false;
        }
        // lparen is not alphanumeric but otherwise `int` would match `print()`
        if (next_char.is_alphanumeric() || next_char == '_') || next_char == '(' {
            return false;
        }

        self.cursor += word.len();
        self.push_tok(tok, start_cursor);
        return true;
    }
    fn lex_arr_def(&mut self, word: &str, base_type: TypeTok) -> Result<bool, ToyError> {
        let start_cursor = self.cursor;
        for (i, c) in word.char_indices() {
            if self.peek(i) != c {
                return Ok(false);
            }
        }

        // don't match keywords that are part of longer identifiers
        let prev_char = if self.cursor == 0 {
            '\0'
        } else {
            self.source_chars[self.cursor - 1]
        };
        if prev_char.is_alphanumeric() || prev_char == '_' {
            return Ok(false);
        }

        let next_char_after_word = self.peek(word.len());
        if next_char_after_word.is_alphanumeric() || next_char_after_word == '_' {
            return Ok(false);
        }

        let mut dimension = 0;
        let mut scan = self.cursor + word.len();
        let source_len = self.source_chars.len();

        while scan + 1 < source_len
            && self.source_chars[scan] == '['
            && self.source_chars[scan + 1] == ']'
        {
            dimension += 1;
            scan += 2;
        }

        if dimension == 0 {
            return Ok(false);
        }

        let arr_type = match base_type {
            TypeTok::Int => TypeTok::IntArr(dimension),
            TypeTok::Bool => TypeTok::BoolArr(dimension),
            TypeTok::Str => TypeTok::StrArr(dimension),
            TypeTok::Float => TypeTok::FloatArr(dimension),
            TypeTok::Any => TypeTok::AnyArr(dimension),
            _ => {
                return Err(ToyError::new(
                    ToyErrorType::ArrayTypeInvalid,
                    self.get_error_span(),
                ));
            }
        };

        self.push_tok(Token::Type(arr_type), start_cursor);
        self.cursor = scan;
        return Ok(true);
    }

    pub fn lex(&mut self, input: String) -> Result<Vec<SpannedToken>, ToyError> {
        self.char_byte_offsets = input.char_indices().map(|(byte_pos, _)| byte_pos).collect();
        self.source_chars = input.chars().collect();
        self.cursor = 0;

        while self.cursor < self.source_chars.len() {
            let c = self.source_chars[self.cursor];
            let tok_start = self.cursor;
            if self.in_string_literal {
                self.eat();
                if c == '"' {
                    self.flush();
                    self.in_string_literal = false;
                    continue;
                }
                if c == '\\' {
                    if self.cursor < self.source_chars.len() {
                        let next_c = self.source_chars[self.cursor];
                        match next_c {
                            'n' => self.string_buffer.push('\n'),
                            't' => self.string_buffer.push('\t'),
                            'r' => self.string_buffer.push('\r'),
                            '\\' => self.string_buffer.push('\\'),
                            '"' => self.string_buffer.push('"'),
                            '0' => self.string_buffer.push('\0'),
                            _ => {
                                self.string_buffer.push('\\');
                                self.string_buffer.push(next_c);
                            }
                        }
                        self.eat();
                    } else {
                        self.string_buffer.push('\\');
                    }
                    continue;
                }
                self.string_buffer.push(c);
                continue;
            }
            let _ = tok_start;
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r') && !self.in_string_literal {
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
            if self.lex_keyword("export", Token::Export) {
                continue;
            }
            if self.lex_keyword("interface", Token::Interface) {
                continue;
            }
            if self.lex_keyword("implements", Token::Implements) {
                continue;
            }
            if (c.is_ascii_digit() || (c == '.' && self.number_buffer.len() > 0))
                && self.string_buffer.len() == 0
            {
                debug!(targets: ["lexer_verbose"], "In ascii print");
                if self.number_buffer.is_empty() {
                    self.buffer_start_cursor = self.cursor;
                }
                self.number_buffer.push(c);
                self.eat();
                continue;
            }
            if c == '.' {
                if self.number_buffer.len() > 0 {
                    self.number_buffer.push(c);
                    self.eat();
                    continue;
                }
                self.flush();
                if self.pending_tokens.len() == 0 {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedFieldName,
                        self.get_error_span(),
                    ));
                }
                match self.pending_tokens.last().unwrap() {
                    Token::VarName(_) | Token::VarRef(_) | Token::RBrack | Token::RParen => {}
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::MalformedFieldName,
                            self.get_error_span(),
                        ));
                    }
                }
                let source_len = self.source_chars.len();
                loop {
                    self.cursor += 1;
                    let mut field_name = String::new();

                    while self.cursor < self.source_chars.len()
                        && (self.source_chars[self.cursor].is_alphanumeric()
                            || self.source_chars[self.cursor] == '_')
                    {
                        field_name.push(self.source_chars[self.cursor]);
                        self.cursor += 1;
                    }

                    if field_name.is_empty() {
                        return Err(ToyError::new(
                            ToyErrorType::MalformedFieldName,
                            self.get_error_span(),
                        ));
                    }

                    let dot_cursor = self.cursor - field_name.len() - 1;
                    let field_start_cursor = dot_cursor + 1;
                    let dot_byte =
                        self.char_byte_offsets.get(dot_cursor).copied().unwrap_or(0) as u64;
                    self.token_start_bytes.push(dot_byte);
                    self.pending_tokens.push(Token::Dot);
                    self.push_tok(Token::VarRef(Box::new(field_name)), field_start_cursor);

                    if self.cursor < source_len && self.source_chars[self.cursor] == '.' {
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
                    self.push_tok(Token::CompoundPlus, tok_start);
                    self.cursor += 2;
                    continue;
                }
                if self.peek(1) == '+' {
                    self.flush();
                    self.push_tok(Token::PlusPlus, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.flush();
                self.push_tok(Token::Plus, tok_start);
                self.eat();
                continue;
            }
            if c == '-' {
                if self.peek(1) == '=' {
                    self.flush();
                    self.push_tok(Token::CompoundMinus, tok_start);
                    self.cursor += 2;
                    continue;
                }
                if self.peek(1) == '-' {
                    self.flush();
                    self.push_tok(Token::MinusMinus, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.flush();
                if self.peek(1).is_ascii_digit() {
                    let is_unary = self.pending_tokens.is_empty() || {
                        match self.pending_tokens.last().unwrap() {
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
                        if self.number_buffer.is_empty() {
                            self.buffer_start_cursor = self.cursor;
                        }
                        self.number_buffer.push('-');
                        self.eat();
                        continue;
                    }
                }
                self.flush();
                self.push_tok(Token::Minus, tok_start);
                self.eat();
                continue;
            }
            if c == '*' {
                if self.peek(1) == '=' {
                    self.flush();
                    self.push_tok(Token::CompoundMultiply, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.flush();
                self.push_tok(Token::Multiply, tok_start);
                self.eat();
                continue;
            }
            if c == '/' {
                if self.peek(1) == '/' {
                    self.flush();
                    //Comment, skip to end of line
                    while self.cursor < self.source_chars.len()
                        && self.source_chars[self.cursor] != '\n'
                    {
                        self.eat();
                    }
                    continue;
                }
                if self.peek(1) == '=' {
                    self.flush();
                    self.push_tok(Token::CompoundDivide, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.flush();
                self.push_tok(Token::Divide, tok_start);
                self.eat();
                continue;
            }
            if c == '(' {
                self.flush();
                self.push_tok(Token::LParen, tok_start);
                self.eat();
                continue;
            }
            if c == ')' {
                self.flush();
                self.push_tok(Token::RParen, tok_start);
                self.eat();
                continue;
            }
            if c == '%' {
                self.flush();
                self.push_tok(Token::Modulo, tok_start);
                self.eat();
                continue;
            }
            if c == '&' && self.peek(1) == '&' {
                self.flush();
                self.push_tok(Token::And, tok_start);
                self.cursor += 2;
                continue;
            }
            if c == '|' && self.peek(1) == '|' {
                self.flush();
                self.push_tok(Token::Or, tok_start);
                self.cursor += 2;
                continue;
            }
            if c == '<' {
                self.flush();
                if self.peek(1) == '=' {
                    self.push_tok(Token::LessThanEqt, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.push_tok(Token::LessThan, tok_start);
                self.eat();
                continue;
            }
            if c == '>' {
                self.flush();
                if self.peek(1) == '=' {
                    self.push_tok(Token::GreaterThanEqt, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.push_tok(Token::GreaterThan, tok_start);
                self.eat();
                continue;
            }
            if c == '=' {
                self.flush();
                if self.peek(1) == '=' {
                    self.push_tok(Token::Equals, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.push_tok(Token::Assign, tok_start);
                self.eat();
                continue;
            }
            if c == '"' {
                if self.in_string_literal {
                    self.flush_str();
                    self.in_string_literal = false;
                } else {
                    self.flush();
                    self.buffer_start_cursor = self.cursor;
                    self.in_string_literal = true;
                }
                self.eat();
                continue;
            }
            if c == '!' {
                self.flush();
                if self.peek(1) == '=' {
                    self.push_tok(Token::NotEquals, tok_start);
                    self.cursor += 2;
                    continue;
                }
                self.push_tok(Token::Not, tok_start);
                self.eat();
                continue;
            }
            if c == '{' {
                self.flush();
                self.push_tok(Token::LBrace, tok_start);
                self.eat();
                continue;
            }
            if c == '}' {
                self.flush();
                self.push_tok(Token::RBrace, tok_start);
                self.eat();
                continue;
            }
            if c == ';' {
                self.flush();
                self.push_tok(Token::Semicolon, tok_start);
                self.eat();
                continue;
            }
            if c == ':' {
                self.flush();
                self.push_tok(Token::Colon, tok_start);
                self.eat();
                continue;
            }
            if c == ',' {
                self.flush();
                self.push_tok(Token::Comma, tok_start);
                self.eat();
                continue;
            }
            if c == '[' {
                self.flush();
                self.push_tok(Token::LBrack, tok_start);
                self.eat();
                continue;
            }
            if c == ']' {
                self.flush();
                self.push_tok(Token::RBrack, tok_start);
                self.eat();
                continue;
            }
            if c.is_ascii() {
                self.flush_num();
                if self.string_buffer.is_empty() {
                    self.buffer_start_cursor = self.cursor;
                }
                self.string_buffer.push(c);
                self.eat();
                continue;
            }

            return Err(ToyError::new(
                ToyErrorType::UnknownCharacter(c),
                self.get_error_span(),
            ));
        }
        debug!(targets: ["lexer_verbose"], self.pending_tokens.clone());

        // flush any trailing numeric literal
        self.flush_num();
        // build SpannedTokens: each token spans [its_start, next_token_start)
        let file_path = Driver::get_current_file_path().unwrap_or_else(|| "<unknown>".to_string());
        let source_end_byte = self.char_byte_offsets.last().copied().unwrap_or(0) as u64;
        let starts = self.token_start_bytes.clone();
        let spanned: Vec<SpannedToken> = self
            .pending_tokens
            .iter()
            .enumerate()
            .map(|(i, tok)| {
                let start = starts[i] as i64;
                let end = starts.get(i + 1).copied().unwrap_or(source_end_byte) as i64;
                SpannedToken {
                    tok: tok.clone(),
                    span: Span::new(&file_path, start, end.max(start)),
                }
            })
            .collect();
        self.clean_up();
        return Ok(spanned);
    }
    fn eat(&mut self) {
        self.cursor += 1;
    }
    fn flush(&mut self) {
        self.flush_num();
        self.flush_str();
    }
    fn flush_num(&mut self) {
        if self.number_buffer.is_empty() {
            return;
        }
        let raw: String = self.number_buffer.iter().collect();
        let start = self.buffer_start_cursor;
        self.number_buffer = Vec::new();
        if raw.contains('.') {
            let value: f64 = raw.parse().unwrap();
            self.push_tok(Token::FloatLit(OrderedFloat(value)), start);
        } else {
            let value: i64 = raw.parse().unwrap();
            self.push_tok(Token::IntLit(value), start);
        }
    }
    fn flush_str(&mut self) {
        if self.in_string_literal {
            let text: String = self.string_buffer.iter().collect();
            self.push_tok(Token::StringLit(Box::new(text)), self.buffer_start_cursor);
            self.string_buffer = Vec::new();
            return;
        }

        if self.string_buffer.is_empty() {
            return;
        }

        let start = self.buffer_start_cursor;
        let text: String = self.string_buffer.iter().collect();
        self.string_buffer = Vec::new();

        // Struct keyword was just pushed with an empty name — fill it in
        if !self.pending_tokens.is_empty()
            && self.pending_tokens.last().unwrap() == &Token::Struct(Box::new("".to_string()))
        {
            let last = self.pending_tokens.len() - 1;
            self.pending_tokens[last] = Token::Struct(Box::new(text));
            // span slot was already reserved by push_tok for the Struct token
            return;
        }

        if self.pending_tokens.is_empty() {
            self.push_tok(Token::VarRef(Box::new(text)), start);
            return;
        }

        let last_type = self.pending_tokens.last().unwrap().tok_type();
        if last_type == "Let" || last_type == "Func" || last_type == "Import" {
            self.push_tok(Token::VarName(Box::new(text)), start);
        } else {
            self.push_tok(Token::VarRef(Box::new(text)), start);
        }
    }

    fn clean_up(&mut self) {
        self.source_chars = Vec::new();
        self.cursor = 0;
        self.number_buffer = Vec::new();
        self.string_buffer = Vec::new();
        self.pending_tokens = Vec::new();
        self.token_start_bytes = Vec::new();
        self.buffer_start_cursor = 0;
    }
}

//Loads and executes tests
#[cfg(test)]
mod tests;
