use crate::token::Token;
use crate::debug;
#[derive(Debug)]
pub struct Lexer {
    chars: Vec<char>,
    cp: usize, //Char pointer
    num_buf: Vec<char>,
    toks: Vec<Token>,
}
impl Lexer {
    pub fn new() -> Lexer {
        let c_vec: Vec<char> = Vec::new();
        let n_vec: Vec<char> = Vec::new();
        let t_vec: Vec<Token> = Vec::new();
        return Lexer {
            chars: c_vec,
            cp: 0usize,
            num_buf: n_vec,
            toks: t_vec,
        };
    }
    pub fn lex(&mut self, input: String) -> Vec<Token> {
        self.chars = input.chars().collect();
        self.cp = 0;

        while self.cp < self.chars.len() {
            let c = self.chars[self.cp];
            debug!(c);
            debug!(self.cp);
            if c == ' ' || c == '\t' || c == '\n' || c == '\r'{
                self.eat();
                continue;
            }
            if c.is_ascii_digit() {
                debug!("In ascii print");
                self.num_buf.push(c);
                self.eat();
                continue;
            }
            if c == '+' {
                self.flush_int();
                self.toks.push(Token::Plus);
                self.eat();
                continue;
            }
            if c == '-'{
                self.flush_int();
                self.toks.push(Token::Minus);
                self.eat();
                continue;
            }
            if c == '*'{
                self.flush_int();
                self.toks.push(Token::Multiply);
                self.eat();
                continue;
            }
            if c == '/'{
                self.flush_int();
                self.toks.push(Token::Divide);
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
    fn flush_int(&mut self){
        if self.num_buf.len() == 0{
            return;
        }
        let proto_output: String = self.num_buf.clone().into_iter().collect();
        let output: i64 = proto_output.parse().unwrap();
        self.num_buf = Vec::new();
        self.toks.push(Token::IntLit(output));
    }
    fn clean_up(&mut self){
        self.chars = Vec::new();
        self.cp = 0;
        self.num_buf = Vec::new();
        self.toks = Vec::new();
    }
}

//Loads and executes tests
#[cfg(test)]
mod tests;
