use crate::token::{Token, Operator};


#[derive(Debug)]
pub struct Lexer{
    chars: Vec<char>,
    cp: usize, //Char pointer
}
impl Lexer{
    pub fn new() -> Lexer{
        let c_vec: Vec<char> = Vec::new();
        return Lexer { 
            chars: c_vec,
            cp: 0usize, 
        }
    }
    pub fn lex(&mut self, input: String) ->Vec<Token>{
        self.chars = input.chars().collect();
        self.cp = 0;

        return vec![];
    }
    fn peek(&self, idx: usize) -> char{
        return self.chars[self.cp + idx];
    }
}
