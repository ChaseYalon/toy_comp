use crate::parser::ast::Ast;
use crate::parser::ast::InfixOp;
use crate::parser::toy_box::TBox;
use crate::token::Token;
use std::collections::HashMap;
pub struct AstGenerator {
    boxes: Vec<TBox>,
    nodes: Vec<Ast>,
    bp: usize,
    //Static lifetime to make red line go away
    p_table: HashMap<&'static str, u32>,
}
impl AstGenerator {
    pub fn new() -> AstGenerator {
        let b_vec: Vec<TBox> = Vec::new();
        let n_vec: Vec<Ast> = Vec::new();
        let mut map: HashMap<&str, u32> = HashMap::new();

        map.insert(Token::Plus.tok_type(), 1);
        map.insert(Token::Minus.tok_type(), 1);
        map.insert(Token::Multiply.tok_type(), 2);
        map.insert(Token::Divide.tok_type(), 2);
        map.insert(Token::IntLit(0).tok_type(), 100); //Zero is just an arbitrary value, it will be fine for all int literals

        return AstGenerator {
            boxes: b_vec,
            nodes: n_vec,
            bp: 0_usize,
            p_table: map,
        };
    }
    fn parse_int_expr(&self, toks: &Vec<Token>) -> Ast {
        if toks.len() == 1 {
            if toks[0].tok_type() == "IntLit"{

                return Ast::IntLit(toks[0].get_val().unwrap());
            }
        }
        let mut best_idx = 0_usize;
        let mut best_val: u32 = 100000000; //Practical infinity
        let mut best_tok: Token = Token::IntLit(0); //This will throw an error later if its val has not been changed
        for (i, t) in toks.iter().enumerate() {
            if t.tok_type() == "IntLit" {
                continue;
            }
            let maybe_val = self.p_table.get(t.tok_type());

            if !maybe_val.is_some() {
                panic!("[ERROR] Unknown symbol, got {}", t);
            }
            let val = *maybe_val.unwrap();

            if val < best_val {
                best_val = val;
                best_idx = i;
                best_tok = t.clone();
            }
        }
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let l_node = self.parse_int_expr(&left.to_vec());
        let r_node = self.parse_int_expr(&right.to_vec());
        return Ast::InfixExpr(
            Box::new(l_node),
            Box::new(r_node),
            match best_tok {
                Token::Plus => InfixOp::Plus,
                Token::Minus => InfixOp::Minus,
                Token::Multiply => InfixOp::Multiply,
                Token::Divide => InfixOp::Divide,
                _ => panic!("[ERROR] WTF happen here"),
            },
        );
    }
    pub fn eat(&mut self) {
        self.bp += 1;
    }
    pub fn generate(&mut self, boxes: Vec<TBox>) -> Vec<Ast> {
        self.boxes = boxes.clone();
        self.bp = 0_usize;

        while self.bp < self.boxes.len() {
            let val = &self.boxes[self.bp];
            match val {
                TBox::IntExpr(i) => {
                    let node = self.parse_int_expr(i);
                    self.nodes.push(node);
                    self.eat();
                }
            }
        }
        return self.nodes.clone();
    }
}

#[cfg(test)]
mod test;