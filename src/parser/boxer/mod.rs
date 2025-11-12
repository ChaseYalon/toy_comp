use crate::debug;
use crate::parser::toy_box::TBox;
use crate::token::{Token, TypeTok};
use std::collections::HashMap;
pub struct Boxer {
    toks: Vec<Token>,
    tp: usize, // token pointer
    interfaces: HashMap<String, HashMap<String, TypeTok>>,
}

impl Boxer {
    pub fn new() -> Boxer {
        Boxer {
            toks: Vec::new(),
            tp: 0,
            interfaces: HashMap::new(),
        }
    }
    fn pre_process(&self, input: &Vec<Token>) -> Vec<Token> {
        let mut toks: Vec<Token> = Vec::new();
        for (i, t) in input.iter().enumerate() {
            if t.tok_type() == "CompoundPlus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Plus);
                continue;
            }
            if t.tok_type() == "CompoundMinus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Minus);
                continue;
            }
            if t.tok_type() == "CompoundMultiply" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Multiply);
                continue;
            }
            if t.tok_type() == "CompoundDivide" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Divide);
                continue;
            }
            if t.tok_type() == "PlusPlus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Plus);
                toks.push(Token::IntLit(1));
                continue;
            }
            if t.tok_type() == "CompoundMinus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Minus);
                toks.push(Token::IntLit(1));
                continue;
            }
            toks.push(t.clone());
        }
        return toks;
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
                Token::VarRef(v) => {
                    let temp = self.interfaces.get(&*v).unwrap().clone();
                    let boxed: HashMap<String, Box<TypeTok>> = temp
                        .clone()
                        .into_iter()
                        .map(|(k, v)| (k, Box::new(v)))
                        .collect();
                    TypeTok::Struct(boxed)
                } //assume it is a nested struct
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
    fn box_while_stmt(&mut self, input: &Vec<Token>) -> TBox {
        if input[0].tok_type() != "While" {
            panic!("[ERROR] Expected a while token, got {}", input[0].clone());
        }

        let mut cond: Vec<Token> = Vec::new();
        let mut brace_start_idx = None;

        for (i, t) in input.iter().enumerate().skip(1) {
            if t.tok_type() == "LBrace" {
                brace_start_idx = Some(i);
                break;
            }
            cond.push(t.clone());
        }

        let brace_start_idx =
            brace_start_idx.unwrap_or_else(|| panic!("[ERROR] Expected '{{' in while statement"));

        if input.last().unwrap().tok_type() != "RBrace" {
            panic!("[ERROR] Expected closing brace in while statement");
        }

        let body_toks = input[brace_start_idx + 1..input.len() - 1].to_vec();
        let boxed_body = self.box_group(body_toks);

        TBox::While(cond, boxed_body)
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
            if ty == "While" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone()));
                    curr.clear();
                }

                let mut while_end = i + 1;

                while while_end < input.len() && input[while_end].tok_type() != "LBrace" {
                    while_end += 1;
                }

                if while_end >= input.len() {
                    panic!("[ERROR] While loop missing body");
                }

                let mut depth = 1;
                while_end += 1; // Move past the opening brace

                while while_end < input.len() && depth > 0 {
                    if input[while_end].tok_type() == "LBrace" {
                        depth += 1;
                    } else if input[while_end].tok_type() == "RBrace" {
                        depth -= 1;
                    }
                    while_end += 1;
                }

                if depth != 0 {
                    panic!("[ERROR] Unterminated while loop");
                }

                let while_slice = input[i..while_end].to_vec();
                boxes.push(self.box_while_stmt(&while_slice));
                i = while_end;
                continue;
            }
            if ty == "Func" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone()));
                    curr.clear();
                }

                let mut func_end = i + 1;
                let mut depth = 0;
                let mut found_body = false;

                for j in i..input.len() {
                    if input[j].tok_type() == "LBrace" {
                        depth = 1;
                        found_body = true;
                        func_end = j + 1;
                        break;
                    }
                }

                if !found_body {
                    panic!("[ERROR] Function declaration missing body");
                }

                while func_end < input.len() && depth > 0 {
                    if input[func_end].tok_type() == "LBrace" {
                        depth += 1;
                    } else if input[func_end].tok_type() == "RBrace" {
                        depth -= 1;
                    }
                    func_end += 1;
                }

                let func_slice = input[i..func_end].to_vec();
                boxes.push(self.box_fn_stmt(func_slice));
                i = func_end;
                continue;
            }

            curr.push(t);
            i += 1;
        }

        if !curr.is_empty() {
            boxes.push(self.box_statement(curr.clone()));
        }

        return boxes;
    }

    fn box_params(&mut self, input: Vec<Token>) -> Vec<TBox> {
        //The structure of the input should be VarRef, Colon, Type, comma
        if input.len() < 3 {
            //no params
            return [].to_vec();
        }
        //Split by comma
        let triplets: Vec<&[Token]> = input.as_slice().split(|t| *t == Token::Comma).collect();
        let mut func_params: Vec<TBox> = Vec::new();
        for triple in triplets {
            if triple[0].tok_type() != "VarRef" {
                panic!("[ERROR] Expected VarRef got {}", triple[0]);
            }
            if triple[1].tok_type() != "Colon" {
                panic!("[ERROR] Expected Colon, got {}", triple[1]);
            }
            if triple[2].tok_type() != "Type" {
                panic!("[ERROR] Expected a type, got {}", triple[2]);
            }
            let param = TBox::FuncParam(
                triple[0].clone(),
                match triple[2].clone() {
                    Token::Type(tok) => tok,
                    _ => unreachable!(),
                },
            );
            func_params.push(param);
        }
        return func_params;
    }

    fn box_fn_stmt(&mut self, input: Vec<Token>) -> TBox {
        if input[0].tok_type() != "Func" {
            panic!("[ERROR] Expected \"fn\" got {}", input[0]);
        }
        let func_name = input[1].clone();
        if input[2].tok_type() != "LParen" {
            panic!("[ERROR] Expected \"(\" got {}", input[2]);
        }
        let mut unboxed_params: Vec<Token> = Vec::new();
        let mut return_type_begin: usize = 0;
        for i in 3..input.len() {
            if input[i].tok_type() == "RParen" {
                return_type_begin = i;
                break;
            }
            unboxed_params.push(input[i].clone());
        }
        let boxed_params: Vec<TBox> = self.box_params(unboxed_params);

        if input[return_type_begin + 1].tok_type() != "Colon" {
            panic!(
                "[ERROR] Expected colon after function params, got {}",
                input[return_type_begin + 1]
            );
        }
        if input[return_type_begin + 2].tok_type() != "Type" {
            panic!(
                "[ERROR] Expected return type, got {}",
                input[return_type_begin + 2]
            );
        }
        if input[return_type_begin + 3].tok_type() != "LBrace" {
            panic!(
                "[ERROR] Expected Opening brace, got {}",
                input[return_type_begin + 3]
            );
        }
        let body_toks = &input[return_type_begin + 4..input.len() - 1];

        if input.last().unwrap().tok_type() != "RBrace" {
            panic!(
                "[ERROR] Expected closing brace, got {}, input({:?})",
                input.last().unwrap(),
                input.clone()
            );
        }
        let body_boxes: Vec<TBox> = self.box_group(body_toks.to_vec());
        let return_type = match input[return_type_begin + 2].clone() {
            Token::Type(t) => t,
            _ => panic!(
                "[ERROR] Expected type, got {}",
                input[return_type_begin + 2]
            ),
        };
        return TBox::FuncDec(func_name, boxed_params, return_type, body_boxes);
    }
    fn box_struct_interface_dec(&mut self, toks: &Vec<Token>) -> TBox {
        if toks[0].tok_type() != "Struct" {
            panic!("[ERROR] Expected struct, got {}", toks[0]);
        }
        let name = match toks[0].clone() {
            Token::Struct(n) => *n,
            _ => unreachable!(),
        };
        if toks[1].tok_type() != "LBrace" {
            panic!("[ERROR] Expected \"{{\", got {}", toks[1]);
        }
        let item_groups: Vec<&[Token]> = toks[2..toks.len() - 1]
            .split(|item| item == &Token::Comma)
            .collect();
        let mut params: HashMap<String, TypeTok> = HashMap::new();

        for group in item_groups {
            if group[1] != Token::Colon {
                panic!(
                    "[ERROR] Names and types must be separated by colon, got {}",
                    group[1]
                );
            }
            let key: String = match group[0].clone() {
                Token::VarRef(v) => *v,
                _ => unreachable!(),
            };
            let value: TypeTok = match group[2].clone() {
                Token::Type(t) => t,
                Token::VarRef(v) => {
                    let temp = self.interfaces.get(&*v).unwrap().clone();
                    let boxed: HashMap<String, Box<TypeTok>> = temp
                        .clone()
                        .into_iter()
                        .map(|(k, v)| (k, Box::new(v)))
                        .collect();
                    TypeTok::Struct(boxed)
                } //assume it is a nested struct
                _ => panic!("[ERROR] Expected Type, got {}", group[2].clone()),
            };
            params.insert(key, value);
        }
        self.interfaces.insert(name.clone(), params.clone());

        return TBox::StructInterface(Box::new(name), Box::new(params));
    }
    fn box_statement(&mut self, toks: Vec<Token>) -> TBox {
        if toks.is_empty() {
            panic!("[ERROR] Empty statement encountered");
        }

        let first = toks[0].tok_type();

        if first == "Let" {
            return self.box_var_dec(&toks);
        }
        if first == "Struct" {
            return self.box_struct_interface_dec(&toks);
        }
        if first == "If" {
            let (stmt, _) = self.box_if_standalone(&toks);
            return stmt;
        }
        if first == "Return" {
            return TBox::Return(Box::new(TBox::Expr(toks[1..toks.len()].to_vec())));
        }
        if first == "Break" {
            return TBox::Break;
        }
        if first == "Continue" {
            return TBox::Continue;
        }
        if toks.len() > 2 && toks[0].tok_type() == "VarRef" && toks[1].tok_type() == "Assign" {
            return self.box_var_ref(&toks);
        }
        if toks.len() > 2 && toks[0].tok_type() == "VarRef" && toks[1].tok_type() == "LBrack" {
            let mut idx_groups: Vec<Vec<Token>> = Vec::new();
            let mut i = 2;
            let len = toks.len();

            while i < len {
                let mut idx_toks: Vec<Token> = Vec::new();

                while i < len && toks[i].tok_type() != "RBrack" {
                    idx_toks.push(toks[i].clone());
                    i += 1;
                }

                if i >= len {
                    panic!("[ERROR] Unclosed array index brackets");
                }

                idx_groups.push(idx_toks);
                i += 1;

                if i < len && toks[i].tok_type() == "LBrack" {
                    i += 1;
                    continue;
                } else {
                    break;
                }
            }

            if i >= len || toks[i].tok_type() != "Assign" {
                panic!("[ERROR] Expected '=', got {:?}", toks);
            }
            let val_toks = toks[i + 1..].to_vec();

            return TBox::ArrReassign(toks[0].clone(), idx_groups, val_toks);
        }
        if toks.len() > 1 && first == "StructRef" && toks[1].tok_type() == "Assign" {
            //struct reassign like a.x = 0;
            let (struct_name, field_names) = match toks[0].clone() {
                Token::StructRef(sn, fen) => (*sn, fen),
                _ => unreachable!(),
            };
            let to_reassign_toks = toks[2..toks.len()].to_vec();
            return TBox::StructReassign(Box::new(struct_name), field_names, to_reassign_toks);
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
        self.toks = self.pre_process(&input);
        //self.toks = input.clone();
        self.tp = 0;
        self.box_group(self.toks.clone())
    }
}

#[cfg(test)]
mod tests;
