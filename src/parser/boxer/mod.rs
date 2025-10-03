use crate::parser::toy_box::TBox;
use crate::token::Token;
pub struct Boxer {
    toks: Vec<Token>,
    tp: usize, //Token pointer
}

impl Boxer {
    pub fn new() -> Boxer {
        let t_vec: Vec<Token> = Vec::new();
        return Boxer {
            toks: t_vec,
            tp: 0_usize,
        };
    }
    pub fn box_toks(&mut self, input: Vec<Token>) -> Vec<TBox> {
        self.toks = input.clone();
        self.tp = 0;

        //TODO: Add more cases for more types of tokens, for now everything is an expression so we can just return it in one box
        return vec![TBox::IntExpr(self.toks.clone())];
    }
}

//Load test
#[cfg(test)]
mod tests;
