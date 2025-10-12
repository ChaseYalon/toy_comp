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
    fn split_into_groups(&self) -> Vec<Vec<Token>>{
        let mut toks: Vec<Vec<Token>> = Vec::new();
        let mut curr_toks: Vec<Token> = Vec::new();
        for tok in self.toks.clone(){
            if tok.tok_type() == "Semicolon"{
                toks.push(curr_toks);
                curr_toks = Vec::new();
                continue;
            }
            curr_toks.push(tok);
        }
        if !curr_toks.is_empty(){
            toks.push(curr_toks);
        }
        return toks;
    }
    fn box_var_dec(&self, input: &Vec<Token>) -> TBox{
        if input[0].tok_type() != "Let"{
            panic!("[ERROR] Variable declaration must start with let, got {}", input[0]);
        }
        if input[1].tok_type() != "VarName"{
            panic!("[ERROR] Let must be followed by variable name, got {}", input[1]);
        }
        if input[2].tok_type() != "Assign" && input[2].tok_type() != "Colon"{
            panic!("[ERROR] Variable time must be followed by an equals sign, or a colon got {}", input[2]);
        }
        if input[2].tok_type() == "Colon" {
            let ty = match input[3].clone() {
                Token::Type(t) => t,
                _ => panic!("[ERROR] Expected type, found {}", input[3])
            };
            return TBox::VarDec(
                input[1].clone(),
                Some(ty),
                input[5..].to_vec()
            )
        }
        return TBox::VarDec(
            input[1].clone(), 
            None,
            input[3..].to_vec()
        );
    }
    fn box_var_ref(&self, input: &Vec<Token>) -> TBox{
        if input[0].tok_type() != "VarRef" {
            panic!("[ERROR] Variable reassign must start with variable reference, got {}", input[0]);
        }
        if input[1].tok_type() != "Assign" {
            panic!("[ERROR] Variable reassign must have variable reference followed by equals sign, got {}", input[1]);
        }
        return TBox::VarReassign(
            input[0].clone(),
            input[2..].to_vec()
        );
    }
    pub fn box_toks(&mut self, input: Vec<Token>) -> Vec<TBox> {
        self.toks = input.clone();
        self.tp = 0;
        let mut boxes: Vec<TBox> = Vec::new();
        let groups = self.split_into_groups();
        for group in groups{
            if group[0].tok_type() == "Let"{
                boxes.push(self.box_var_dec(&group));
                continue;
            }
            if group.len() > 2 {
                if group[0].tok_type() == "VarRef" && group[1].tok_type() == "Assign" {
                    boxes.push(self.box_var_ref(&group));
                    continue;
                }
            }

            //Assume it is an expression
            boxes.push(TBox::IntExpr(group.clone()));
        }
        return boxes;
    }
}

//Load test
#[cfg(test)]
mod tests;
