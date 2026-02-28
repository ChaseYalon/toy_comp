use crate::errors::{Span, ToyError, ToyErrorType};
use crate::parser::toy_box::TBox;
use crate::token::{SpannedToken, Token, TypeTok};
use std::collections::BTreeMap;
pub struct Boxer {
    toks: Vec<SpannedToken>,
    tp: usize, // token pointer
    interfaces: BTreeMap<String, BTreeMap<String, TypeTok>>,
    /// Optional module prefix for name mangling (e.g., "std::math")
    module_prefix: Option<String>,
    current_struct: Option<(String, TypeTok)>,
}

impl Boxer {
    pub fn new() -> Boxer {
        Boxer {
            toks: Vec::new(),
            tp: 0,
            interfaces: BTreeMap::new(),
            module_prefix: None,
            current_struct: None,
        }
    }

    /// Create a Boxer with a module prefix for name mangling.
    /// The prefix should be in the form "std::<path>::<filename>" (e.g., "std::math")
    pub fn with_module_prefix(prefix: String) -> Boxer {
        Boxer {
            toks: Vec::new(),
            tp: 0,
            interfaces: BTreeMap::new(),
            module_prefix: Some(prefix),
            current_struct: None,
        }
    }
    pub fn total_span(toks: Vec<SpannedToken>)-> Span {
        let first = toks[0].span.clone();
        //SAFETY - this array should always be filled
        let last = toks.last().unwrap().span.clone();
        return Span::new(&first.file_path, first.start_offset_bytes, last.end_offset_bytes);
    }
    /// Mangle a function name with the module prefix if set
    fn parse_type(&self, input: &[SpannedToken]) -> Result<(TypeTok, usize), ToyError> {
        if input.is_empty() {
            return Err(ToyError::new(
                ToyErrorType::MalformedType,
                Span::null_span_with_msg(
                    &"Parse_type received an empty input. This should be impossible.",
                ),
            ));
        }
        let cumulative_toks = Boxer::total_span(input.to_vec());
        match &input[0].tok {
            Token::Type(t) => {
                let mut dim = 0;
                let mut i = 1;
                while i + 1 < input.len()
                    && input[i].tok == Token::LBrack
                    && input[i + 1].tok == Token::RBrack
                {
                    dim += 1;
                    i += 2;
                }
                if dim == 0 {
                    return Ok((t.clone(), 1));
                }
                let new_type = match t {
                    TypeTok::Int => TypeTok::IntArr(dim),
                    TypeTok::Bool => TypeTok::BoolArr(dim),
                    TypeTok::Str => TypeTok::StrArr(dim),
                    TypeTok::Float => TypeTok::FloatArr(dim),
                    TypeTok::Any => TypeTok::AnyArr(dim),
                    _ => return Err(ToyError::new(ToyErrorType::MalformedType, input[0].span.clone())),
                };
                return Ok((new_type, i));
            }
            Token::VarRef(v) | Token::VarName(v) => {
                let struct_fields = self.interfaces.get(v.as_ref()).ok_or_else(|| {
                    ToyError::new(ToyErrorType::MalformedType, cumulative_toks.clone())
                })?;
                let mut dim = 0;
                let mut i = 1;
                while i + 1 < input.len()
                    && input[i].tok == Token::LBrack
                    && input[i + 1].tok == Token::RBrack
                {
                    dim += 1;
                    i += 2;
                }
                let boxed_fields: BTreeMap<String, Box<TypeTok>> = struct_fields
                    .clone()
                    .into_iter()
                    .map(|(k, v)| (k, Box::new(v)))
                    .collect();
                if dim > 0 {
                    Ok((TypeTok::StructArr(boxed_fields, dim), i))
                } else {
                    Ok((TypeTok::Struct(boxed_fields), 1))
                }
            }
            _ => Err(ToyError::new(
                ToyErrorType::MalformedType,
                cumulative_toks
                //Some(format!("Expected type, found: {}", input[0])),
            )),
        }
    }
    fn mangle_func_name(&self, name: Token) -> Token {
        if let Some(prefix) = &self.module_prefix {
            if let Some(func_name) = name.get_var_name() {
                return Token::VarName(Box::new(format!("{}::{}", prefix, func_name)));
            }
        }
        name
    }
    fn box_var_dec(&self, input: &Vec<SpannedToken>) -> Result<TBox, ToyError> {
        let cumulative_span = Boxer::total_span(input.clone());
        if input[0].tok.tok_type() != "Let" {
            return Err(ToyError::new(
                ToyErrorType::MalformedLetStatement,
                cumulative_span,
            ));
        }
        if input[1].tok.tok_type() != "VarName" {
            return Err(ToyError::new(
                ToyErrorType::MalformedLetStatement,
                cumulative_span,
            ));
        }
        if input[2].tok.tok_type() != "Assign" && input[2].tok.tok_type() != "Colon" {
            return Err(ToyError::new(
                ToyErrorType::MalformedLetStatement,
                cumulative_span,
            ));
        }
        if input[2].tok.tok_type() == "Colon" {
            let (ty, consumed) = self.parse_type(&input[3..])?;
            let val_start = 3 + consumed;
            if input[val_start].tok.tok_type() != "Assign" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedLetStatement,
                    cumulative_span,
                ));
            }
            return Ok(TBox::VarDec(
                input[1].clone(),
                Some(ty),
                input[val_start + 1..].to_vec(),
                cumulative_span,
            ));
        }
        return Ok(TBox::VarDec(
            input[1].clone(),
            None,
            input[3..].to_vec(),
            cumulative_span,
        ));
    }

    fn box_while_stmt(&mut self, input: &Vec<SpannedToken>) -> Result<TBox, ToyError> {
        let cumulative_span = Boxer::total_span(input.clone());
        if input[0].tok.tok_type() != "While" {
            return Err(ToyError::new(
                ToyErrorType::MalformedWhileStatement,
                cumulative_span,
            ));
        }

        let mut cond: Vec<SpannedToken> = Vec::new();
        let mut brace_start_idx = None;

        for (i, t) in input.iter().enumerate().skip(1) {
            if t.tok.tok_type() == "LBrace" {
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
                    cumulative_span,
                ));
            }
        };

        if input.last().unwrap().tok.tok_type() != "RBrace" {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                cumulative_span,
            ));
        }

        let body_toks = input[brace_start_idx + 1..input.len() - 1].to_vec();
        let boxed_body = self.box_group(body_toks);

        return Ok(TBox::While(cond, boxed_body?, cumulative_span));
    }

    fn box_group(&mut self, input: Vec<SpannedToken>) -> Result<Vec<TBox>, ToyError> {
        let cumulative_span = Boxer::total_span(input.clone());
        let mut boxes: Vec<TBox> = Vec::new();
        let mut curr: Vec<SpannedToken> = Vec::new();
        let mut brace_depth = 0;
        let mut paren_depth = 0;
        let mut i = 0;

        while i < input.len() {
            let t = input[i].clone();
            let ty = t.tok.tok_type();

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

                while while_end < input.len() && input[while_end].tok.tok_type() != "LBrace" {
                    while_end += 1;
                }

                if while_end >= input.len() {

                    return Err(ToyError::new(
                        ToyErrorType::MalformedWhileStatement,
                        cumulative_span,
                    ));
                }

                let mut depth = 1;
                while_end += 1; // Move past the opening brace

                while while_end < input.len() && depth > 0 {
                    if input[while_end].tok.tok_type() == "LBrace" {
                        depth += 1;
                    } else if input[while_end].tok.tok_type() == "RBrace" {
                        depth -= 1;
                    }
                    while_end += 1;
                }

                if depth != 0 {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        cumulative_span,
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

                while for_end < input.len() && input[for_end].tok.tok_type() != "LBrace" {
                    for_end += 1;
                }

                if for_end >= input.len() {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedStructInterface,
                        cumulative_span,
                    ));
                }

                let depth = 1;
                for_end += 1; // Move past the opening brace

                if depth != 0 {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        cumulative_span,
                    ));
                }

                let for_slice = input[i..for_end].to_vec();
                let mut new_boxes = self.box_for_block(&for_slice)?;
                boxes.append(&mut new_boxes);
                i = for_end;
                continue;
            }
            if (ty == "Func" || ty == "Extern" || ty == "Export")
                && brace_depth == 0
                && paren_depth == 0
            {
                if !curr.is_empty() {
                    boxes.push(self.box_statement(curr.clone())?);
                    curr.clear();
                }

                // Check if this is an export declaration
                let is_export = ty == "Export";
                let actual_start = if is_export { i + 1 } else { i };

                // If export, verify the next token is Func or Extern
                if is_export {
                    if actual_start >= input.len() {
                        return Err(ToyError::new(
                            ToyErrorType::MalformedFunctionDeclaration,
                            Span::null_span_with_msg("export must be followed by fn or extern")//this seems like it should have line:col info
                        ));
                    }
                    let next_type = input[actual_start].tok.tok_type();
                    if next_type != "Func" && next_type != "Extern" {
                        return Err(ToyError::new(
                            ToyErrorType::MalformedFunctionDeclaration,
                            Span::null_span_with_msg("export must be followed by fn or extern"), //this seems like it should have line:col info
                        ));
                    }
                }

                if (is_export
                    && actual_start < input.len()
                    && input[actual_start].tok.tok_type() == "Extern")
                    || ty == "Extern"
                {
                    let mut func_end = actual_start + 1;
                    while func_end < input.len() && input[func_end].tok.tok_type() != "Semicolon" {
                        func_end += 1;
                    }
                    if func_end >= input.len() {

                        return Err(ToyError::new(
                            ToyErrorType::MalformedFunctionDeclaration,
                            cumulative_span,
                        ));
                    }
                    // Include the semicolon
                    func_end += 1;
                    let func_slice = input[actual_start..func_end].to_vec();
                    boxes.push(self.box_extern_fn_stmt(func_slice)?);
                    i = func_end;
                    continue;
                }

                let mut func_end = actual_start + 1;
                let mut depth = 0;
                let mut found_body = false;

                for j in actual_start..input.len() {
                    if input[j].tok.tok_type() == "LBrace" {
                        depth = 1;
                        found_body = true;
                        func_end = j + 1;
                        break;
                    }
                }

                if !found_body {

                    return Err(ToyError::new(
                        ToyErrorType::MalformedFunctionDeclaration,
                        cumulative_span,
                    ));
                }

                while func_end < input.len() && depth > 0 {
                    if input[func_end].tok.tok_type() == "LBrace" {
                        depth += 1;
                    } else if input[func_end].tok.tok_type() == "RBrace" {
                        depth -= 1;
                    }
                    func_end += 1;
                }

                let func_slice = input[actual_start..func_end].to_vec();
                boxes.push(self.box_fn_stmt(func_slice, is_export)?);
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

    fn box_params(&mut self, input: Vec<SpannedToken>) -> Result<Vec<TBox>, ToyError> {
        let cumulative_span = if input.is_empty() {
            Span::null_span_with_msg(&"empty input to box_params")
        } else {
            Boxer::total_span(input.clone())
        };
        //The structure of the input should be VarRef, Colon, Type, comma
        if input.len() < 3 {
            //no params
            return Ok(vec![]);
        }
        //Split by comma
        let triplets: Vec<&[SpannedToken]> =
            input.as_slice().split(|t| t.tok == Token::Comma).collect();
        let mut func_params: Vec<TBox> = Vec::new();
        for triple in triplets {

            let (param_type, _) = self.parse_type(&triple[2..])?;
            let param = TBox::FuncParam(triple[0].clone(), param_type, cumulative_span.clone());
            func_params.push(param);
        }
        return Ok(func_params);
    }

    fn box_extern_fn_stmt(&mut self, input: Vec<SpannedToken>) -> Result<TBox, ToyError> {
        let cumulative_span = Boxer::total_span(input.clone());
        // input[0] is Extern
        if input[0].tok.tok_type() != "Extern" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                cumulative_span.clone(),
            ));
        }
        // input[1] should be Func
        if input[1].tok.tok_type() != "Func" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                cumulative_span.clone(),
            ));
        }

        let func_name = input[2].clone();
        if input[3].tok.tok_type() != "LParen" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                cumulative_span.clone(),
            ));
        }

        let mut unboxed_params: Vec<SpannedToken> = Vec::new();
        let mut return_type_begin: usize = 0;
        for i in 4..input.len() {
            if input[i].tok.tok_type() == "RParen" {
                return_type_begin = i;
                break;
            }
            unboxed_params.push(input[i].clone());
        }
        let boxed_params: Vec<TBox> = self.box_params(unboxed_params)?;

        // Check return type
        let return_type = if input[return_type_begin + 1].tok.tok_type() == "Semicolon" {
            TypeTok::Void
        } else {
            if input[return_type_begin + 1].tok.tok_type() != "Colon" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    cumulative_span.clone(),
                ));
            }
            let (t, consumed) = self.parse_type(&input[return_type_begin + 2..])?;

            if input[return_type_begin + 2 + consumed].tok.tok_type() != "Semicolon" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedFunctionDeclaration,
                    cumulative_span.clone(),
                ));
            }
            t
        };

        return Ok(TBox::ExternFuncDec(
            func_name,
            boxed_params,
            return_type,
            cumulative_span,
        ));
    }

    fn box_fn_stmt(&mut self, input: Vec<SpannedToken>, is_export: bool) -> Result<TBox, ToyError> {
        let cumulative_span = Boxer::total_span(input.clone());
        if input[0].tok.tok_type() != "Func" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                cumulative_span.clone(),
            ));
        }
        let func_name = if self.current_struct.is_some() {
            input[1].clone()
        } else {
            SpannedToken {
                tok: self.mangle_func_name(input[1].tok.clone()),
                span: input[1].span.clone(),
            }
        };
        if input[2].tok.tok_type() != "LParen" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFunctionDeclaration,
                cumulative_span.clone(),
            ));
        }
        let mut unboxed_params: Vec<SpannedToken> = Vec::new();
        let mut return_type_begin: usize = 0;
        for i in 3..input.len() {
            if input[i].tok.tok_type() == "RParen" {
                return_type_begin = i;
                break;
            }
            unboxed_params.push(input[i].clone());
        }
        let mut boxed_params: Vec<TBox> = self.box_params(unboxed_params)?;

        let mut func_name = func_name;
        if let Some((struct_name, struct_type)) = &self.current_struct {
            let this_param = TBox::FuncParam(
                SpannedToken {
                    tok: Token::VarRef(Box::new("this".to_string())),
                    span: cumulative_span.clone(),
                },
                struct_type.clone(),
                cumulative_span.clone(),
            );
            boxed_params.insert(0, this_param);
            if let Token::VarName(n) = func_name.tok.clone() {
                func_name.tok = Token::VarName(Box::new(format!("{}:::{}", struct_name, n)));
            }
        }

        let (return_type, body_start_idx) =
            if input[return_type_begin + 1].tok.tok_type() == "LBrace" {
                (TypeTok::Void, return_type_begin + 2)
            } else {
                if input[return_type_begin + 1].tok.tok_type() != "Colon" {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedFunctionDeclaration,
                        cumulative_span.clone(),
                    ));
                }
                let (t, consumed) = self.parse_type(&input[return_type_begin + 2..])?;

                if input[return_type_begin + 2 + consumed].tok.tok_type() != "LBrace" {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedFunctionDeclaration,
                        cumulative_span.clone(),
                    ));
                }
                (t, return_type_begin + 2 + consumed + 1)
            };

        let body_toks = &input[body_start_idx..input.len() - 1];

        if input.last().unwrap().tok.tok_type() != "RBrace" {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                cumulative_span.clone(),
            ));
        }
        let mut final_mangled_name = func_name;
        for p in boxed_params.clone() {
            let ty = match p {
                TBox::FuncParam(_, t, _) => t,
                _ => unreachable!(),
            };
            final_mangled_name.tok = Token::VarName(Box::new(format!(
                "{}_{}",
                final_mangled_name.tok,
                ty.type_str().to_lowercase()
            )));
        }
        let body_boxes: Vec<TBox> = self.box_group(body_toks.to_vec())?;
        return Ok(TBox::FuncDec(
            final_mangled_name,
            boxed_params,
            return_type,
            body_boxes,
            cumulative_span,
            is_export,
        ));
    }
    fn box_struct_interface_dec(&mut self, toks: &Vec<SpannedToken>) -> Result<TBox, ToyError> {
        let cumulative_span = Boxer::total_span(toks.to_vec());
        let name = match toks[0].tok.clone() {
            Token::Struct(n) => *n,
            _ => unreachable!(),
        };
        if toks[1].tok.tok_type() != "LBrace" {
            return Err(ToyError::new(
                ToyErrorType::MalformedStructInterface,
                cumulative_span.clone(),
            ));
        }
        let item_groups: Vec<&[SpannedToken]> = toks[2..toks.len() - 1]
            .split(|item| item.tok == Token::Comma)
            .collect();
        let mut params: BTreeMap<String, TypeTok> = BTreeMap::new();

        for group in item_groups {
            if group.is_empty() {
                continue;
            }
            if group[1].tok != Token::Colon {
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructInterface,
                    Boxer::total_span(group.to_vec()),
                ));
            }
            let key: String = match group[0].tok.clone() {
                Token::VarRef(v) => *v,
                _ => unreachable!(),
            };
            let value: TypeTok = match group[2].tok.clone() {
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
                        ToyErrorType::MalformedStructInterface,
                        Boxer::total_span(group.to_vec()),
                    ));
                }
            };
            params.insert(key, value);
        }
        self.interfaces.insert(name.clone(), params.clone());

        return Ok(TBox::StructInterface(
            Box::new(name),
            Box::new(params),
            cumulative_span,
        ));
    }

    fn box_for_block(&mut self, input: &Vec<SpannedToken>) -> Result<Vec<TBox>, ToyError> {
        let cumulative_span = Boxer::total_span(input.to_vec());
        if input.len() < 4 {
            return Err(ToyError::new(
                ToyErrorType::MalformedStructInterface,
                cumulative_span.clone(),
            ));
        }

        let struct_name = match &input[1].tok {
            Token::VarRef(n) => *n.clone(),
            _ => {
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructInterface,
                    cumulative_span.clone(),
                ));
            }
        };

        let struct_fields = match self.interfaces.get(&struct_name) {
            Some(fields) => fields.clone(),
            None => {
                return Err(ToyError::new(
                    ToyErrorType::UndefinedStruct,
                    cumulative_span.clone(),
                ));
            }
        };

        let boxed_fields: BTreeMap<String, Box<TypeTok>> = struct_fields
            .into_iter()
            .map(|(k, v)| (k, Box::new(v)))
            .collect();

        let this_type = TypeTok::Struct(boxed_fields.clone());

        let prefixed_struct_name = if let Some(prefix) = &self.module_prefix {
            format!("{}::{}", prefix, struct_name)
        } else {
            struct_name.clone()
        };
        self.current_struct = Some((prefixed_struct_name, this_type.clone()));
        let body_toks = input[3..input.len() - 1].to_vec();
        let boxed_body = self.box_group(body_toks)?;
        self.current_struct = None;

        Ok(boxed_body)
    }
    fn box_import_stmt(&self, toks: &Vec<SpannedToken>) -> Result<TBox, ToyError> {
        let cumulative_span = Boxer::total_span(toks.to_vec());
        if toks[0].tok.tok_type() != "Import" {
            return Err(ToyError::new(
                ToyErrorType::MalformedImportStatement,
                cumulative_span.clone(),
            ));
        }
        if toks.len() < 2 {
            return Err(ToyError::new(
                ToyErrorType::MalformedImportStatement,
                cumulative_span.clone(),
            ));
        }
        let mut module_name = String::new();
        for i in 1..toks.len() {
            if toks[i].tok.tok_type() == "Semicolon" {
                break;
            }
            module_name.push_str(&toks[i].tok.to_string());
        }
        Ok(TBox::ImportStmt(module_name, cumulative_span))
    }
    fn box_statement(&mut self, toks: Vec<SpannedToken>) -> Result<TBox, ToyError> {
        let first = toks[0].tok.tok_type();
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
            let expr_span = Boxer::total_span(toks[1..toks.len()].to_vec());
            return Ok(TBox::Return(
                Box::new(TBox::Expr(toks[1..toks.len()].to_vec(), expr_span.clone())),
                Boxer::total_span(toks.to_vec()),
            ));
        }
        if first == "Break" {
            return Ok(TBox::Break(Boxer::total_span(toks.to_vec())));
        }
        if first == "Continue" {
            return Ok(TBox::Continue(Boxer::total_span(toks.to_vec())));
        }

        // Check for Assignment (=) or Compound Assignment (+=, -=, *=, /=)
        let mut assign_idx = None;
        let mut compound_op = None;

        for (i, t) in toks.iter().enumerate() {
            match t.tok.tok_type().as_str() {
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
            let cumulative_span = Boxer::total_span(toks.to_vec());

            if lhs.is_empty() {
                return Err(ToyError::new(
                    ToyErrorType::MalformedVariableReassign,
                    cumulative_span.clone(),
                ));
            }

            if let Some(op) = compound_op {
                // Expand lhs += rhs to lhs = lhs + rhs
                // Expand lhs ++ to lhs = lhs + 1
                let mut new_rhs = lhs.clone();
                new_rhs.push(SpannedToken {
                    tok: op,
                    span: cumulative_span.clone(),
                });
                if rhs.is_empty() {
                    new_rhs.push(SpannedToken {
                        tok: Token::IntLit(1),
                        span: cumulative_span.clone(),
                    });
                } else {
                    new_rhs.extend(rhs);
                }
                return Ok(TBox::Assign(lhs, new_rhs, cumulative_span));
            } else {
                return Ok(TBox::Assign(lhs, rhs, cumulative_span));
            }
        }

        let cumulative_span = Boxer::total_span(toks.to_vec());

        return Ok(TBox::Expr(toks, cumulative_span));
    }

    /// Parse an if statement from a token slice, returning the TBox and number of tokens consumed
    fn box_if_standalone(&mut self, input: &Vec<SpannedToken>) -> Result<(TBox, usize), ToyError> {
        let mut i = 1;

        let cumulative_span = Boxer::total_span(input.to_vec());
        let mut cond: Vec<SpannedToken> = Vec::new();
        while i < input.len() && input[i].tok.tok_type() != "LBrace" {
            cond.push(input[i].clone());
            i += 1;
        }

        if i >= input.len() {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                cumulative_span.clone(),
            ));
        }

        i += 1; // skip '{'

        let mut depth = 1;
        let mut body_toks: Vec<SpannedToken> = Vec::new();
        while i < input.len() && depth > 0 {
            let t = input[i].clone();
            if t.tok.tok_type() == "LBrace" {
                depth += 1;
            } else if t.tok.tok_type() == "RBrace" {
                depth -= 1;
            }

            if depth > 0 {
                body_toks.push(t);
            }
            i += 1;
        }

        if depth != 0 {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                cumulative_span.clone(),
            ));
        }

        let body_boxes = self.box_group(body_toks);

        let mut else_ifs: Vec<(Vec<SpannedToken>, Vec<TBox>)> = Vec::new();

        while i < input.len() {
            if input[i].tok.tok_type() == "Else"
                && i + 1 < input.len()
                && input[i + 1].tok.tok_type() == "If"
            {
                i += 2; // skip 'else' 'if'

                let mut elif_cond: Vec<SpannedToken> = Vec::new();
                while i < input.len() && input[i].tok.tok_type() != "LBrace" {
                    elif_cond.push(input[i].clone());
                    i += 1;
                }

                if i >= input.len() {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        cumulative_span.clone(),
                    ));
                }

                i += 1; // skip '{'

                let mut depth = 1;
                let mut elif_body_toks: Vec<SpannedToken> = Vec::new();
                while i < input.len() && depth > 0 {
                    let t = input[i].clone();
                    if t.tok.tok_type() == "LBrace" {
                        depth += 1;
                    } else if t.tok.tok_type() == "RBrace" {
                        depth -= 1;
                    }

                    if depth > 0 {
                        elif_body_toks.push(t);
                    }
                    i += 1;
                }

                if depth != 0 {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        cumulative_span.clone(),
                    ));
                }

                let elif_body_boxes = self.box_group(elif_body_toks)?;
                else_ifs.push((elif_cond, elif_body_boxes));
            } else {
                break;
            }
        }

        let mut else_body_boxes = None;
        if i < input.len() && input[i].tok.tok_type() == "Else" {
            i += 1; // skip 'Else'

            if i >= input.len() || input[i].tok.tok_type() != "LBrace" {
                return Err(ToyError::new(
                    ToyErrorType::UnclosedDelimiter,
                    cumulative_span.clone(),
                ));
            }
            i += 1; // skip '{'

            let mut depth = 1;
            let mut else_toks: Vec<SpannedToken> = Vec::new();
            while i < input.len() && depth > 0 {
                let t = input[i].clone();
                if t.tok.tok_type() == "LBrace" {
                    depth += 1;
                } else if t.tok.tok_type() == "RBrace" {
                    depth -= 1;
                }

                if depth > 0 {
                    else_toks.push(t);
                }
                i += 1;
            }

            if depth != 0 {
                return Err(ToyError::new(
                    ToyErrorType::UnclosedDelimiter,
                    cumulative_span.clone(),
                ));
            }

            else_body_boxes = Some(self.box_group(else_toks)?);
        }

        let elifs = if else_ifs.is_empty() {
            None
        } else {
            Some(else_ifs)
        };

        Ok((
            TBox::IfStmt(
                cond,
                body_boxes?,
                elifs,
                else_body_boxes,
                cumulative_span.clone(),
            ),
            i,
        ))
    }

    /// Recursively box tokens into structured TBoxes (proto-AST)
    pub fn box_toks(&mut self, input: Vec<SpannedToken>) -> Result<Vec<TBox>, ToyError> {
        self.toks = input;
        //self.toks = input.clone();
        self.tp = 0;
        return Ok(self.box_group(self.toks.clone())?);
    }
}

#[cfg(test)]
mod tests;
