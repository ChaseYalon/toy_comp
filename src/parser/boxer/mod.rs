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
    fn box_var_dec(&self, input: &Vec<Token>) -> Result<TBox, ToyError> {
        let raw_text = if input[2].tok_type() == "Colon" {
            let mut s = "".to_string();
            for item in input[5..].to_vec() {
                s = s + &item.to_string();
            }
            format!("let {}: {} = {}", input[1], input[2], s)
        } else {
            let mut s = "".to_string();
            for item in input[3..].to_vec() {
                s = s + &item.to_string();
            }
            format!("let {} = {}", input[1], s)
        };
        if input[0].tok_type() != "Let" {
            return Err(ToyError::new(
                ToyErrorType::MalformedLetStatement,
                Some(raw_text),
            ));
        }
        if input[1].tok_type() != "VarName" {
            return Err(ToyError::new(
                ToyErrorType::MalformedLetStatement,
                Some(raw_text),
            ));
        }
        if input[2].tok_type() != "Assign" && input[2].tok_type() != "Colon" {
            return Err(ToyError::new(
                ToyErrorType::MalformedLetStatement,
                Some(raw_text),
            ));
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
                _ => {
                    return Err(ToyError::new(
                        ToyErrorType::VariableNotAStruct,
                        Some(raw_text),
                    ));
                } //is this the right bug
            };
            return Ok(TBox::VarDec(
                input[1].clone(),
                Some(ty),
                input[5..].to_vec(),
                raw_text,
            ));
        }
        return Ok(TBox::VarDec(
            input[1].clone(),
            None,
            input[3..].to_vec(),
            raw_text,
        ));
    }

    fn box_while_stmt(&mut self, input: &Vec<Token>) -> Result<TBox, ToyError> {
        let raw_text = format!(
            "while {} {{",
            input[1..]
                .iter()
                .take_while(|t| t.tok_type() != "LBrace")
                .map(|t| t.to_string())
                .collect::<String>()
        );
        if input[0].tok_type() != "While" {
            return Err(ToyError::new(
                ToyErrorType::MalformedWhileStatement,
                Some(raw_text),
            ));
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
            None => {
                return Err(ToyError::new(
                    ToyErrorType::UnclosedDelimiter,
                    Some(raw_text.clone()),
                ));
            }
        };

        if input.last().unwrap().tok_type() != "RBrace" {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                Some(raw_text),
            ));
        }

        let body_toks = input[brace_start_idx + 1..input.len() - 1].to_vec();
        let boxed_body = self.box_group(body_toks);

        Ok(TBox::While(cond, boxed_body?, raw_text))
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
                    let raw_text = format!(
                        "while {}",
                        input[i..]
                            .iter()
                            .take(5)
                            .map(|t| t.to_string())
                            .collect::<String>()
                    );
                    return Err(ToyError::new(
                        ToyErrorType::MalformedWhileStatement,
                        Some(raw_text),
                    ));
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
                    let raw_text = format!(
                        "while {}",
                        input[i..while_end.min(i + 10)]
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<String>()
                    );
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        Some(raw_text),
                    ));
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
                    let raw_text = format!(
                        "for {} {{",
                        input[i..]
                            .iter()
                            .take(5)
                            .map(|t| t.to_string())
                            .collect::<String>()
                    );
                    return Err(ToyError::new(
                        ToyErrorType::MalformedStructInterface,
                        Some(raw_text),
                    ));
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
                    let raw_text = format!(
                        "for {}",
                        input[i..for_end.min(i + 10)]
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<String>()
                    );
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        Some(raw_text),
                    ));
                }

                let for_slice = input[i..for_end].to_vec();
                let mut new_boxes = self.box_for_block(&for_slice)?;
                boxes.append(&mut new_boxes);
                i = for_end;
                continue;
            }
            if (ty == "Func" || ty == "Extern") && brace_depth == 0 && paren_depth == 0 {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone())?);
                    curr.clear();
                }

                if ty == "Extern" {
                    let mut func_end = i + 1;
                    while func_end < input.len() && input[func_end].tok_type() != "Semicolon" {
                        func_end += 1;
                    }
                    if func_end >= input.len() {
                        let raw_text = format!(
                            "extern fn {}",
                            input[i..i + 5.min(input.len() - i)]
                                .iter()
                                .map(|t| t.to_string())
                                .collect::<String>()
                        );
                        return Err(ToyError::new(
                            ToyErrorType::MalformedFunctionDeclaration,
                            Some(raw_text),
                        ));
                    }
                    // Include the semicolon
                    func_end += 1;
                    let func_slice = input[i..func_end].to_vec();
                    boxes.push(self.box_extern_fn_stmt(func_slice)?);
                    i = func_end;
                    continue;
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
                    let raw_text = format!(
                        "func {}",
                        input[i..i + 5.min(input.len() - i)]
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<String>()
                    );
                    return Err(ToyError::new(
                        ToyErrorType::MalformedFunctionDeclaration,
                        Some(raw_text),
                    ));
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
                let raw_text = format!(
                    "param: {}",
                    triple.iter().map(|t| t.to_string()).collect::<String>()
                );
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text),
                ));
            }
            if triple[1].tok_type() != "Colon" {
                let raw_text = format!(
                    "param: {}",
                    triple.iter().map(|t| t.to_string()).collect::<String>()
                );
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text),
                ));
            }

            if triple[2].tok_type() != "Type" && triple[2].tok_type() != "VarRef" {
                let raw_text = format!(
                    "param: {}",
                    triple.iter().map(|t| t.to_string()).collect::<String>()
                );
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text),
                ));
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
                format!("{}: {}", triple[0], triple[2]),
            );
            func_params.push(param);
        }
        return Ok(func_params);
    }

    fn box_extern_fn_stmt(&mut self, input: Vec<Token>) -> Result<TBox, ToyError> {
        let raw_text = format!(
            "extern fn {}",
            input
                .iter()
                .take(10)
                .map(|t| t.to_string())
                .collect::<String>()
        );
        // input[0] is Extern
        if input[0].tok_type() != "Extern" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                Some(raw_text.clone()),
            ));
        }
        // input[1] should be Func
        if input[1].tok_type() != "Func" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                Some(raw_text.clone()),
            ));
        }

        let func_name = input[2].clone();
        if input[3].tok_type() != "LParen" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                Some(raw_text),
            ));
        }

        let mut unboxed_params: Vec<Token> = Vec::new();
        let mut return_type_begin: usize = 0;
        for i in 4..input.len() {
            if input[i].tok_type() == "RParen" {
                return_type_begin = i;
                break;
            }
            unboxed_params.push(input[i].clone());
        }
        let boxed_params: Vec<TBox> = self.box_params(unboxed_params)?;

        // Check return type
        // After RParen, we expect Colon Type Semicolon OR Semicolon (Void)

        let return_type = if input[return_type_begin + 1].tok_type() == "Semicolon" {
            TypeTok::Void
        } else {
            if input[return_type_begin + 1].tok_type() != "Colon" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text.clone()),
                ));
            }
            if input[return_type_begin + 2].tok_type() != "Type" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text.clone()),
                ));
            }
            if input[return_type_begin + 3].tok_type() != "Semicolon" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text.clone()),
                ));
            }
            match input[return_type_begin + 2].clone() {
                Token::Type(t) => t,
                _ => unreachable!(),
            }
        };

        return Ok(TBox::ExternFuncDec(
            func_name,
            boxed_params,
            return_type,
            raw_text,
        ));
    }

    fn box_fn_stmt(&mut self, input: Vec<Token>) -> Result<TBox, ToyError> {
        let raw_text = format!(
            "func {}",
            input
                .iter()
                .take(10)
                .map(|t| t.to_string())
                .collect::<String>()
        );
        if input[0].tok_type() != "Func" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                Some(raw_text.clone()),
            ));
        }
        let func_name = input[1].clone();
        if input[2].tok_type() != "LParen" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                Some(raw_text),
            ));
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
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text.clone()),
                ));
            }
            if input[return_type_begin + 2].tok_type() != "Type" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text.clone()),
                ));
            }
            if input[return_type_begin + 3].tok_type() != "LBrace" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    Some(raw_text.clone()),
                ));
            }
            let t = match input[return_type_begin + 2].clone() {
                Token::Type(t) => t,
                _ => {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedFunctionDeclaration,
                        Some(raw_text.clone()),
                    ));
                }
            };
            (t, return_type_begin + 4)
        };

        let body_toks = &input[body_start_idx..input.len() - 1];

        if input.last().unwrap().tok_type() != "RBrace" {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                Some(raw_text),
            ));
        }
        let body_boxes: Vec<TBox> = self.box_group(body_toks.to_vec())?;

        return Ok(TBox::FuncDec(
            func_name,
            boxed_params,
            return_type,
            body_boxes,
            raw_text,
        ));
    }
    fn box_struct_interface_dec(&mut self, toks: &Vec<Token>) -> Result<TBox, ToyError> {
        let name = match toks[0].clone() {
            Token::Struct(n) => *n,
            _ => unreachable!(),
        };
        let raw_text = format!("struct {} {{", name);
        if toks[1].tok_type() != "LBrace" {
            return Err(ToyError::new(
                ToyErrorType::MalformedStructInterface,
                Some(raw_text),
            ));
        }
        let item_groups: Vec<&[Token]> = toks[2..toks.len() - 1]
            .split(|item| item == &Token::Comma)
            .collect();
        let mut params: BTreeMap<String, TypeTok> = BTreeMap::new();

        for group in item_groups {
            if group.is_empty() {
                continue;
            }
            if group[1] != Token::Colon {
                let field_text = format!(
                    "field: {}",
                    group.iter().map(|t| t.to_string()).collect::<String>()
                );
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructInterface,
                    Some(field_text),
                ));
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
                _ => {
                    let field_text = format!(
                        "field: {}",
                        group.iter().map(|t| t.to_string()).collect::<String>()
                    );
                    return Err(ToyError::new(
                        ToyErrorType::MalformedStructInterface,
                        Some(field_text),
                    ));
                }
            };
            params.insert(key, value);
        }
        self.interfaces.insert(name.clone(), params.clone());

        return Ok(TBox::StructInterface(
            Box::new(name),
            Box::new(params),
            raw_text,
        ));
    }

    fn box_for_block(&mut self, input: &Vec<Token>) -> Result<Vec<TBox>, ToyError> {
        let raw_text = format!(
            "for {}",
            input
                .iter()
                .take(5)
                .map(|t| t.to_string())
                .collect::<String>()
        );
        if input.len() < 4 {
            return Err(ToyError::new(
                ToyErrorType::MalformedStructInterface,
                Some(raw_text.clone()),
            ));
        }

        let struct_name = match &input[1] {
            Token::VarRef(n) => *n.clone(),
            _ => {
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructInterface,
                    Some(raw_text.clone()),
                ));
            }
        };

        let struct_fields = match self.interfaces.get(&struct_name) {
            Some(fields) => fields.clone(),
            None => {
                return Err(ToyError::new(
                    ToyErrorType::UndefinedStruct,
                    Some(format!("for {} (undefined struct)", struct_name)),
                ));
            }
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
                TBox::FuncDec(mut name, mut params, ret_type, body, func_raw_text) => {
                    let literal_name = match name {
                        Token::VarName(n) => *n,
                        _ => unreachable!(),
                    };
                    name = Token::VarName(Box::new(struct_name.clone() + ":::" + &literal_name));
                    let this_param = TBox::FuncParam(
                        Token::VarRef(Box::new("this".to_string())),
                        this_type.clone(),
                        "this".to_string(),
                    );
                    params.insert(0, this_param);
                    modified_boxes.push(TBox::FuncDec(name, params, ret_type, body, func_raw_text));
                }
                _ => modified_boxes.push(b),
            }
        }

        Ok(modified_boxes)
    }
    fn box_import_stmt(&self, toks: &Vec<Token>) -> Result<TBox, ToyError> {
        let raw_text = format!(
            "import {};",
            toks[1..]
                .iter()
                .map(|t| t.to_string())
                .collect::<String>()
        );
        if toks[0].tok_type() != "Import" {
            return Err(ToyError::new(
                ToyErrorType::MalformedImportStatement,
                Some(raw_text),
            ));
        }
        if toks.len() < 2 {
            return Err(ToyError::new(
                ToyErrorType::MalformedImportStatement,
                Some(raw_text),
            ));
        }
        let module_name = match toks[1].clone() {
            Token::StringLit(s) => *s,
            Token::VarName(s) => *s,
            Token::VarRef(s) => *s,
            _ => {
                return Err(ToyError::new(
                    ToyErrorType::MalformedImportStatement,
                    Some(raw_text),
                ));
            }
        };
        Ok(TBox::ImportStmt(module_name, raw_text))
    }
    fn box_statement(&mut self, toks: Vec<Token>) -> Result<TBox, ToyError> {
        let first = toks[0].tok_type();
        if first == "Import" {
            return self.box_import_stmt(&toks);
        }
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
            let expr_text = toks[1..toks.len()]
                .iter()
                .map(|t| t.to_string())
                .collect::<String>();
            return Ok(TBox::Return(
                Box::new(TBox::Expr(toks[1..toks.len()].to_vec(), expr_text.clone())),
                format!("return {}", expr_text),
            ));
        }
        if first == "Break" {
            return Ok(TBox::Break);
        }
        if first == "Continue" {
            return Ok(TBox::Continue);
        }

        // Check for Assignment (=) or Compound Assignment (+=, -=, *=, /=)
        let mut assign_idx = None;
        let mut compound_op = None;

        for (i, t) in toks.iter().enumerate() {
            match t.tok_type().as_str() {
                "Assign" => {
                    assign_idx = Some(i);
                    break;
                }
                "CompoundPlus" => {
                    assign_idx = Some(i);
                    compound_op = Some(Token::Plus);
                    break;
                }
                "CompoundMinus" => {
                    assign_idx = Some(i);
                    compound_op = Some(Token::Minus);
                    break;
                }
                "CompoundMultiply" => {
                    assign_idx = Some(i);
                    compound_op = Some(Token::Multiply);
                    break;
                }
                "CompoundDivide" => {
                    assign_idx = Some(i);
                    compound_op = Some(Token::Divide);
                    break;
                }
                "PlusPlus" => {
                    assign_idx = Some(i);
                    compound_op = Some(Token::Plus);
                    break;
                }
                "MinusMinus" => {
                    assign_idx = Some(i);
                    compound_op = Some(Token::Minus);
                    break;
                }
                _ => {}
            }
        }

        if let Some(idx) = assign_idx {
            let lhs = toks[0..idx].to_vec();
            let rhs = toks[idx + 1..].to_vec();
            let raw_text = toks.iter().map(|t| t.to_string()).collect::<String>();

            if lhs.is_empty() {
                return Err(ToyError::new(
                    ToyErrorType::MalformedVariableReassign,
                    Some(raw_text),
                ));
            }

            if let Some(op) = compound_op {
                // Expand lhs += rhs to lhs = lhs + rhs
                // Expand lhs ++ to lhs = lhs + 1
                let mut new_rhs = lhs.clone();
                new_rhs.push(op);
                if rhs.is_empty() {
                    new_rhs.push(Token::IntLit(1));
                } else {
                    new_rhs.extend(rhs);
                }
                return Ok(TBox::Assign(lhs, new_rhs, raw_text));
            } else {
                return Ok(TBox::Assign(lhs, rhs, raw_text));
            }
        }

        let raw_text = toks.iter().map(|t| t.to_string()).collect::<String>();

        return Ok(TBox::Expr(toks, raw_text));
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
            let raw_text = format!(
                "if {}",
                cond.iter().map(|t| t.to_string()).collect::<String>()
            );
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                Some(raw_text),
            ));
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
            let raw_text = format!(
                "if {} {{",
                cond.iter().map(|t| t.to_string()).collect::<String>()
            );
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                Some(raw_text),
            ));
        }

        let body_boxes = self.box_group(body_toks);

        let mut else_body_boxes = None;
        if i < input.len() && input[i].tok_type() == "Else" {
            i += 1; // skip 'Else'

            if i >= input.len() || input[i].tok_type() != "LBrace" {
                let raw_text = format!(
                    "if {} {{ ... }} else",
                    cond.iter().map(|t| t.to_string()).collect::<String>()
                );
                return Err(ToyError::new(
                    ToyErrorType::UnclosedDelimiter,
                    Some(raw_text),
                ));
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
                let raw_text = format!(
                    "if {} {{ ... }} else {{",
                    cond.iter().map(|t| t.to_string()).collect::<String>()
                );
                return Err(ToyError::new(
                    ToyErrorType::UnclosedDelimiter,
                    Some(raw_text),
                ));
            }

            else_body_boxes = Some(self.box_group(else_toks)?);
        }
        let raw_text = format!(
            "if {} {{ ... }}",
            cond.iter().map(|t| t.to_string()).collect::<String>()
        );

        Ok((
            TBox::IfStmt(cond, body_boxes?, else_body_boxes, raw_text),
            i,
        ))
    }

    /// Recursively box tokens into structured TBoxes (proto-AST)
    pub fn box_toks(&mut self, input: Vec<Token>) -> Result<Vec<TBox>, ToyError> {
        self.toks = input;
        //self.toks = input.clone();
        self.tp = 0;
        return Ok(self.box_group(self.toks.clone())?);
    }
}

#[cfg(test)]
mod tests;
