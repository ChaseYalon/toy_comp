use crate::debug;
use crate::parser::toy_box::TBox;
use crate::token::Token;

pub struct Boxer {
    toks: Vec<Token>,
    tp: usize, // token pointer
}

impl Boxer {
    pub fn new() -> Boxer {
        Boxer {
            toks: Vec::new(),
            tp: 0,
        }
    }

    fn box_var_dec(&self, input: &Vec<Token>) -> TBox {
        if input[0].tok_type() != "Let" {
            panic!(
                "[ERROR] Variable declaration must start with let, got {}",
                input[0]
            );
        }
        if input[1].tok_type() != "VarName" {
            panic!(
                "[ERROR] Let must be followed by variable name, got {}",
                input[1]
            );
        }
        if input[2].tok_type() != "Assign" && input[2].tok_type() != "Colon" {
            panic!(
                "[ERROR] Variable declaration must have '=' or ':' after name, got {}",
                input[2]
            );
        }
        if input[2].tok_type() == "Colon" {
            let ty = match input[3].clone() {
                Token::Type(t) => t,
                _ => panic!("[ERROR] Expected type, found {}", input[3]),
            };
            return TBox::VarDec(input[1].clone(), Some(ty), input[5..].to_vec());
        }
        TBox::VarDec(input[1].clone(), None, input[3..].to_vec())
    }

    fn box_var_ref(&self, input: &Vec<Token>) -> TBox {
        if input[0].tok_type() != "VarRef" {
            panic!(
                "[ERROR] Variable reassign must start with variable reference, got {}",
                input[0]
            );
        }
        if input[1].tok_type() != "Assign" {
            panic!(
                "[ERROR] Variable reassign must have '=' after variable reference, got {}",
                input[1]
            );
        }
        TBox::VarReassign(input[0].clone(), input[2..].to_vec())
    }

    fn box_group(&mut self, input: Vec<Token>) -> Vec<TBox> {
        let mut boxes: Vec<TBox> = Vec::new();
        let mut curr: Vec<Token> = Vec::new();
        let mut brace_depth = 0;
        let mut paren_depth = 0;
        let mut i = 0;

        while i < input.len() {
            let t = input[i].clone();
            let ty = t.tok_type();

            if ty == "LBrace" {
                brace_depth += 1;
            } else if ty == "RBrace" {
                brace_depth -= 1;
            } else if ty == "LParen" {
                paren_depth += 1;
            } else if ty == "RParen" {
                paren_depth -= 1;
            }

            if ty == "Semicolon" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone()));
                    curr.clear();
                }
                i += 1;
                continue;
            }

            if ty == "If" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone()));
                    curr.clear();
                }

                let if_slice = input[i..].to_vec();
                let (stmt, consumed) = self.box_if_standalone(&if_slice);
                boxes.push(stmt);

                i += consumed;
                continue;
            }

            curr.push(t);
            i += 1;
        }

        if !curr.is_empty() {
            boxes.push(self.box_statement(curr.clone()));
        }

        boxes
    }

    fn box_statement(&mut self, toks: Vec<Token>) -> TBox {
        if toks.is_empty() {
            panic!("[ERROR] Empty statement encountered");
        }

        let first = toks[0].tok_type();

        if first == "Let" {
            return self.box_var_dec(&toks);
        }

        if first == "If" {
            let (stmt, _) = self.box_if_standalone(&toks);
            return stmt;
        }

        if toks.len() > 2 && toks[0].tok_type() == "VarRef" && toks[1].tok_type() == "Assign" {
            return self.box_var_ref(&toks);
        }

        TBox::Expr(toks)
    }

    /// Parse an if statement from a token slice, returning the TBox and number of tokens consumed
    fn box_if_standalone(&mut self, input: &Vec<Token>) -> (TBox, usize) {
        debug!(targets: ["parser"], input);

        if input.is_empty() || input[0].tok_type() != "If" {
            panic!("[ERROR] Expected 'if' statement");
        }

        let mut i = 1;

        let mut cond: Vec<Token> = Vec::new();
        while i < input.len() && input[i].tok_type() != "LBrace" {
            cond.push(input[i].clone());
            i += 1;
        }

        if i >= input.len() {
            panic!("[ERROR] Expected '{{' after if condition, got end of input");
        }

        i += 1; // skip '{'

        let mut depth = 1;
        let mut body_toks: Vec<Token> = Vec::new();
        while i < input.len() && depth > 0 {
            let t = input[i].clone();
            if t.tok_type() == "LBrace" {
                depth += 1;
            } else if t.tok_type() == "RBrace" {
                depth -= 1;
            }

            if depth > 0 {
                body_toks.push(t);
            }
            i += 1;
        }

        if depth != 0 {
            panic!("[ERROR] Unterminated '{{' block in if statement");
        }

        let body_boxes = self.box_group(body_toks);

        let mut else_body_boxes = None;
        if i < input.len() && input[i].tok_type() == "Else" {
            i += 1; // skip 'Else'

            if i >= input.len() || input[i].tok_type() != "LBrace" {
                panic!("[ERROR] Expected '{{' after 'else'");
            }
            i += 1; // skip '{'

            let mut depth = 1;
            let mut else_toks: Vec<Token> = Vec::new();
            while i < input.len() && depth > 0 {
                let t = input[i].clone();
                if t.tok_type() == "LBrace" {
                    depth += 1;
                } else if t.tok_type() == "RBrace" {
                    depth -= 1;
                }

                if depth > 0 {
                    else_toks.push(t);
                }
                i += 1;
            }

            if depth != 0 {
                panic!("[ERROR] Unterminated '{{' block in else statement");
            }

            else_body_boxes = Some(self.box_group(else_toks));
        }

        (TBox::IfStmt(cond, body_boxes, else_body_boxes), i)
    }

    /// Recursively box tokens into structured TBoxes (proto-AST)
    pub fn box_toks(&mut self, input: Vec<Token>) -> Vec<TBox> {
        self.toks = input.clone();
        self.tp = 0;
        self.box_group(self.toks.clone())
    }
}

#[cfg(test)]
mod tests;
