use crate::errors::{ToyError, ToyErrorType};
use crate::parser::toy_box::TBox;
use crate::token::{Token, TypeTok};
use std::collections::BTreeMap;

pub struct Boxer {
    toks: Vec<Token>,
    tp: usize, // token pointer
    interfaces: BTreeMap<String, BTreeMap<String, TypeTok>>,
}

impl Boxer {
    pub fn new() -> Boxer {
        Boxer {
            toks: Vec::new(),
            tp: 0,
            interfaces: BTreeMap::new(),
        }
    }
    fn pre_process(&self, input: &Vec<Token>) -> Vec<Token> {
        let mut toks: Vec<Token> = Vec::new();
        let mut i = 0usize;
        while i < input.len() {
            let t = input[i].clone();
            // compound ops
            if t.tok_type() == "CompoundPlus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Plus);
                i += 1;
                continue;
            }
            if t.tok_type() == "CompoundMinus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Minus);
                i += 1;
                continue;
            }
            if t.tok_type() == "CompoundMultiply" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Multiply);
                i += 1;
                continue;
            }
            if t.tok_type() == "CompoundDivide" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Divide);
                i += 1;
                continue;
            }
            if t.tok_type() == "PlusPlus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Plus);
                toks.push(Token::IntLit(1));
                i += 1;
                continue;
            }
            if t.tok_type() == "CompoundMinus" {
                toks.push(Token::Assign);
                toks.push(input[i - 1].clone());
                toks.push(Token::Minus);
                toks.push(Token::IntLit(1));
                i += 1;
                continue;
            }

            // collapse dotted field access sequences into StructRef tokens
            if i + 2 < input.len()
                && input[i].tok_type() == "VarRef"
                && input[i + 1].tok_type() == "Dot"
                && input[i + 2].tok_type() == "VarRef"
            {
                if let Token::VarRef(name) = input[i].clone() {
                    let s_name = *name;
                    let mut keys: Vec<String> = Vec::new();
                    i += 1; // move to the dot
                    while i + 1 < input.len()
                        && input[i].tok_type() == "Dot"
                        && input[i + 1].tok_type() == "VarRef"
                    {
                        if let Token::VarRef(k) = input[i + 1].clone() {
                            keys.push(*k);
                        }
                        i += 2;
                    }
                    toks.push(Token::StructRef(Box::new(s_name), keys));
                    continue;
                }
            }

            toks.push(t);
            i += 1;
        }
        return toks;
    }
    fn box_var_dec(&self, input: &Vec<Token>) -> Result<TBox, ToyError> {
        if input[0].tok_type() != "Let" {
            return Err(ToyError::new(ToyErrorType::MalformedLetStatement));
        }
        if input[1].tok_type() != "VarName" {
            return Err(ToyError::new(ToyErrorType::MalformedLetStatement));
        }
        if input[2].tok_type() != "Assign" && input[2].tok_type() != "Colon" {
            return Err(ToyError::new(ToyErrorType::MalformedLetStatement));
        }
        if input[2].tok_type() == "Colon" {
            let ty = match input[3].clone() {
                Token::Type(t) => t,
                Token::VarRef(v) => {
                    let temp = self.interfaces.get(&*v).unwrap().clone();
                    let boxed: BTreeMap<String, Box<TypeTok>> = temp
                        .clone()
                        .into_iter()
                        .map(|(k, v)| (k, Box::new(v)))
                        .collect();
                    TypeTok::Struct(boxed)
                } //assume it is a nested struct
                _ => return Err(ToyError::new(ToyErrorType::VariableNotAStruct)), //is this the right bug
            };
            return Ok(TBox::VarDec(
                input[1].clone(),
                Some(ty),
                input[5..].to_vec(),
            ));
        }
        return Ok(TBox::VarDec(input[1].clone(), None, input[3..].to_vec()));
    }

    fn box_var_ref(&self, input: &Vec<Token>) -> Result<TBox, ToyError> {
        if input[0].tok_type() != "VarRef" {
            return Err(ToyError::new(ToyErrorType::MalformedVariableReassign));
        }
        if input[1].tok_type() != "Assign" {
            return Err(ToyError::new(ToyErrorType::MalformedVariableReassign));
        }
        Ok(TBox::VarReassign(input[0].clone(), input[2..].to_vec()))
    }
    fn box_while_stmt(&mut self, input: &Vec<Token>) -> Result<TBox, ToyError> {
        if input[0].tok_type() != "While" {
            return Err(ToyError::new(ToyErrorType::MalformedWhileStatement));
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

        let brace_start_idx = match brace_start_idx.clone() {
            Some(i) => i,
            None => return Err(ToyError::new(ToyErrorType::UnclosedDelimiter)),
        };

        if input.last().unwrap().tok_type() != "RBrace" {
            return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
        }

        let body_toks = input[brace_start_idx + 1..input.len() - 1].to_vec();
        let boxed_body = self.box_group(body_toks);

        Ok(TBox::While(cond, boxed_body?))
    }

    fn box_group(&mut self, input: Vec<Token>) -> Result<Vec<TBox>, ToyError> {
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
                    boxes.push(self.box_statement(curr.clone())?);
                    curr.clear();
                }
                i += 1;
                continue;
            }

            if ty == "If" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone())?);
                    curr.clear();
                }

                let if_slice = input[i..].to_vec();
                let (stmt, consumed) = self.box_if_standalone(&if_slice)?;
                boxes.push(stmt);

                i += consumed;
                continue;
            }
            if ty == "While" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone())?);
                    curr.clear();
                }

                let mut while_end = i + 1;

                while while_end < input.len() && input[while_end].tok_type() != "LBrace" {
                    while_end += 1;
                }

                if while_end >= input.len() {
                    return Err(ToyError::new(ToyErrorType::MalformedWhileStatement));
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
                    return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
                }

                let while_slice = input[i..while_end].to_vec();
                boxes.push(self.box_while_stmt(&while_slice)?);
                i = while_end;
                continue;
            }
            if ty == "For" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone())?);
                    curr.clear();
                }

                let mut for_end = i + 1;

                while for_end < input.len() && input[for_end].tok_type() != "LBrace" {
                    for_end += 1;
                }

                if for_end >= input.len() {
                    return Err(ToyError::new(ToyErrorType::MalformedStructInterface));
                }

                let mut depth = 1;
                for_end += 1; // Move past the opening brace

                while for_end < input.len() && depth > 0 {
                    if input[for_end].tok_type() == "LBrace" {
                        depth += 1;
                    } else if input[for_end].tok_type() == "RBrace" {
                        depth -= 1;
                    }
                    for_end += 1;
                }

                if depth != 0 {
                    return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
                }

                let for_slice = input[i..for_end].to_vec();
                let mut new_boxes = self.box_for_block(&for_slice)?;
                boxes.append(&mut new_boxes);
                i = for_end;
                continue;
            }
            if ty == "Func" && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone())?);
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
                    return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
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
                boxes.push(self.box_fn_stmt(func_slice)?);
                i = func_end;
                continue;
            }

            curr.push(t);
            i += 1;
        }

        if !curr.is_empty() {
            boxes.push(self.box_statement(curr.clone())?);
        }

        return Ok(boxes);
    }

    fn box_params(&mut self, input: Vec<Token>) -> Result<Vec<TBox>, ToyError> {
        //The structure of the input should be VarRef, Colon, Type, comma
        if input.len() < 3 {
            //no params
            return Ok(vec![]);
        }
        //Split by comma
        let triplets: Vec<&[Token]> = input.as_slice().split(|t| *t == Token::Comma).collect();
        let mut func_params: Vec<TBox> = Vec::new();
        for triple in triplets {
            if triple[0].tok_type() != "VarRef" {
                return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
            }
            if triple[1].tok_type() != "Colon" {
                return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
            }

            if triple[2].tok_type() != "Type" && triple[2].tok_type() != "VarRef" {
                return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
            }
            let param = TBox::FuncParam(
                triple[0].clone(),
                match triple[2].clone() {
                    Token::Type(tok) => tok,
                    Token::VarRef(v) => {
                        let unboxed = self.interfaces.get(&*v).unwrap().clone();
                        let boxed: BTreeMap<String, Box<TypeTok>> = unboxed
                            .clone()
                            .into_iter()
                            .map(|(k, v)| (k, Box::new(v)))
                            .collect();
                        TypeTok::Struct(boxed)
                    }
                    _ => unreachable!(),
                },
            );
            func_params.push(param);
        }
        return Ok(func_params);
    }

    fn box_fn_stmt(&mut self, input: Vec<Token>) -> Result<TBox, ToyError> {
        if input[0].tok_type() != "Func" {
            return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
        }
        let func_name = input[1].clone();
        if input[2].tok_type() != "LParen" {
            return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
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
        let boxed_params: Vec<TBox> = self.box_params(unboxed_params)?;

        let (return_type, body_start_idx) = if input[return_type_begin + 1].tok_type() == "LBrace" {
            (TypeTok::Void, return_type_begin + 2)
        } else {
            if input[return_type_begin + 1].tok_type() != "Colon" {
                return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
            }
            if input[return_type_begin + 2].tok_type() != "Type" {
                return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
            }
            if input[return_type_begin + 3].tok_type() != "LBrace" {
                return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration));
            }
            let t = match input[return_type_begin + 2].clone() {
                Token::Type(t) => t,
                _ => return Err(ToyError::new(ToyErrorType::MalformedFunctionDeclaration)),
            };
            (t, return_type_begin + 4)
        };

        let body_toks = &input[body_start_idx..input.len() - 1];

        if input.last().unwrap().tok_type() != "RBrace" {
            return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
        }
        let body_boxes: Vec<TBox> = self.box_group(body_toks.to_vec())?;

        return Ok(TBox::FuncDec(
            func_name,
            boxed_params,
            return_type,
            body_boxes,
        ));
    }
    fn box_struct_interface_dec(&mut self, toks: &Vec<Token>) -> Result<TBox, ToyError> {
        let name = match toks[0].clone() {
            Token::Struct(n) => *n,
            _ => unreachable!(),
        };
        if toks[1].tok_type() != "LBrace" {
            return Err(ToyError::new(ToyErrorType::MalformedStructInterface));
        }
        let item_groups: Vec<&[Token]> = toks[2..toks.len() - 1]
            .split(|item| item == &Token::Comma)
            .collect();
        let mut params: BTreeMap<String, TypeTok> = BTreeMap::new();

        for group in item_groups {
            if group[1] != Token::Colon {
                return Err(ToyError::new(ToyErrorType::MalformedStructInterface));
            }
            let key: String = match group[0].clone() {
                Token::VarRef(v) => *v,
                _ => unreachable!(),
            };
            let value: TypeTok = match group[2].clone() {
                Token::Type(t) => t,
                Token::VarRef(v) => {
                    let temp = self.interfaces.get(&*v).unwrap().clone();
                    let boxed: BTreeMap<String, Box<TypeTok>> = temp
                        .clone()
                        .into_iter()
                        .map(|(k, v)| (k, Box::new(v)))
                        .collect();
                    TypeTok::Struct(boxed)
                } //assume it is a nested struct
                _ => return Err(ToyError::new(ToyErrorType::MalformedStructInterface)),
            };
            params.insert(key, value);
        }
        self.interfaces.insert(name.clone(), params.clone());

        return Ok(TBox::StructInterface(Box::new(name), Box::new(params)));
    }

    fn box_for_block(&mut self, input: &Vec<Token>) -> Result<Vec<TBox>, ToyError> {
        if input.len() < 4 {
            return Err(ToyError::new(ToyErrorType::MalformedStructInterface));
        }

        let struct_name = match &input[1] {
            Token::VarRef(n) => *n.clone(),
            _ => return Err(ToyError::new(ToyErrorType::MalformedStructInterface)),
        };

        let struct_fields = match self.interfaces.get(&struct_name) {
            Some(fields) => fields.clone(),
            None => return Err(ToyError::new(ToyErrorType::UndefinedStruct)),
        };

        let boxed_fields: BTreeMap<String, Box<TypeTok>> = struct_fields
            .into_iter()
            .map(|(k, v)| (k, Box::new(v)))
            .collect();

        let this_type = TypeTok::Struct(boxed_fields);

        let body_toks = input[3..input.len() - 1].to_vec();
        let boxed_body = self.box_group(body_toks)?;

        let mut modified_boxes = Vec::new();

        for b in boxed_body {
            match b {
                TBox::FuncDec(mut name, mut params, ret_type, body) => {
                    let literal_name = match name {
                        Token::VarName(n) => *n,
                        _ => unreachable!(),
                    };
                    name = Token::VarName(Box::new(struct_name.clone() + ":::" + &literal_name));
                    let this_param = TBox::FuncParam(
                        Token::VarRef(Box::new("this".to_string())),
                        this_type.clone(),
                    );
                    params.insert(0, this_param);
                    modified_boxes.push(TBox::FuncDec(name, params, ret_type, body));
                }
                _ => modified_boxes.push(b),
            }
        }

        Ok(modified_boxes)
    }

    fn box_statement(&mut self, toks: Vec<Token>) -> Result<TBox, ToyError> {
        let first = toks[0].tok_type();

        if first == "Let" {
            return self.box_var_dec(&toks);
        }
        if first == "Struct" {
            return self.box_struct_interface_dec(&toks);
        }
        if first == "If" {
            let (stmt, _) = self.box_if_standalone(&toks)?;
            return Ok(stmt);
        }
        if first == "Return" {
            return Ok(TBox::Return(Box::new(TBox::Expr(
                toks[1..toks.len()].to_vec(),
            ))));
        }
        if first == "Break" {
            return Ok(TBox::Break);
        }
        if first == "Continue" {
            return Ok(TBox::Continue);
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
                    return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
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
                return Err(ToyError::new(ToyErrorType::MalformedVariableReassign)); //this might be wrong
            }
            let val_toks = toks[i + 1..].to_vec();

            return Ok(TBox::ArrReassign(toks[0].clone(), idx_groups, val_toks));
        }
        if toks.len() > 1 && first == "StructRef" && toks[1].tok_type() == "Assign" {
            //struct reassign like a.x = 0;
            let (struct_name, field_names) = match toks[0].clone() {
                Token::StructRef(sn, fen) => (*sn, fen),
                _ => unreachable!(),
            };
            let to_reassign_toks = toks[2..toks.len()].to_vec();
            return Ok(TBox::StructReassign(
                Box::new(struct_name),
                field_names,
                to_reassign_toks,
            ));
        }

        if toks.len() > 1 && first == "StructRef" && toks[1].tok_type() == "LParen" {
            let (struct_name, field_names) = match toks[0].clone() {
                Token::StructRef(sn, fen) => (*sn, fen),
                _ => unreachable!(),
            };

            if !field_names.is_empty() {
                let method_name = field_names.last().unwrap().clone();
                let object_tok = if field_names.len() == 1 {
                    Token::VarRef(Box::new(struct_name))
                } else {
                    Token::StructRef(
                        Box::new(struct_name),
                        field_names[0..field_names.len() - 1].to_vec(),
                    )
                };

                let mut new_toks = Vec::new();
                new_toks.push(Token::VarRef(Box::new(method_name)));
                new_toks.push(Token::LParen);
                new_toks.push(object_tok);

                if toks[2].tok_type() != "RParen" {
                    new_toks.push(Token::Comma);
                }

                new_toks.extend_from_slice(&toks[2..]);

                return Ok(TBox::Expr(new_toks));
            }
        }

        return Ok(TBox::Expr(toks));
    }

    /// Parse an if statement from a token slice, returning the TBox and number of tokens consumed
    fn box_if_standalone(&mut self, input: &Vec<Token>) -> Result<(TBox, usize), ToyError> {
        let mut i = 1;

        let mut cond: Vec<Token> = Vec::new();
        while i < input.len() && input[i].tok_type() != "LBrace" {
            cond.push(input[i].clone());
            i += 1;
        }

        if i >= input.len() {
            return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
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
            return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
        }

        let body_boxes = self.box_group(body_toks);

        let mut else_body_boxes = None;
        if i < input.len() && input[i].tok_type() == "Else" {
            i += 1; // skip 'Else'

            if i >= input.len() || input[i].tok_type() != "LBrace" {
                return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
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
                return Err(ToyError::new(ToyErrorType::UnclosedDelimiter));
            }

            else_body_boxes = Some(self.box_group(else_toks)?);
        }

        Ok((TBox::IfStmt(cond, body_boxes?, else_body_boxes), i))
    }

    /// Recursively box tokens into structured TBoxes (proto-AST)
    pub fn box_toks(&mut self, input: Vec<Token>) -> Result<Vec<TBox>, ToyError> {
        self.toks = self.pre_process(&input);
        //self.toks = input.clone();
        self.tp = 0;
        return Ok(self.box_group(self.toks.clone())?);
    }
}

#[cfg(test)]
mod tests;
