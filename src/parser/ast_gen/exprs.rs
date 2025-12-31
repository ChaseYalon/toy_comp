use super::AstGenerator;
use crate::debug;
use crate::errors::{ToyError, ToyErrorType};
use crate::parser::ast::{Ast, InfixOp};
use crate::token::{Token, TypeTok};
use std::collections::BTreeMap;

impl AstGenerator {
    pub fn parse_num_expr(&self, toks: &Vec<Token>) -> Result<(Ast, TypeTok), ToyError> {
        if toks.len() == 1 {
            if toks[0].tok_type() == "IntLit" {
                return Ok((Ast::IntLit(toks[0].get_val().unwrap()), TypeTok::Int));
            }
            if toks[0].tok_type() == "VarRef" {
                let name = match &toks[0] {
                    Token::VarRef(n) => n,
                    _ => unreachable!(),
                };
                let raw_text = toks
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
                let ty = self.lookup_var_type(name).ok_or_else(|| {
                    ToyError::new(ToyErrorType::TypeHintNeeded, Some(raw_text.clone()))
                })?;
                return Ok((self.parse_var_ref(&toks[0])?, ty));
            }
            if toks[0].tok_type() == "FloatLit" {
                let val = match toks[0] {
                    Token::FloatLit(f) => f,
                    _ => unreachable!(),
                };
                return Ok((Ast::FloatLit(val), TypeTok::Float));
            }
        }
        if toks.len() == 0 {
            return Err(ToyError::new(ToyErrorType::ExpectedExpression, None));
        }
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        let (best_idx, _, best_tok) = self.find_top_val(toks)?;
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, l_type) = self.parse_expr(&left.to_vec())?;
        let (r_node, r_type) = self.parse_expr(&right.to_vec())?;

        let res_type = if l_type == TypeTok::Float || r_type == TypeTok::Float {
            TypeTok::Float
        } else {
            TypeTok::Int
        };

        return Ok((
            Ast::InfixExpr(
                Box::new(l_node),
                Box::new(r_node),
                match best_tok {
                    Token::Plus => InfixOp::Plus,
                    Token::Minus => InfixOp::Minus,
                    Token::Multiply => InfixOp::Multiply,
                    Token::Divide => InfixOp::Divide,
                    Token::Modulo => InfixOp::Modulo,
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::InvalidInfixOperation,
                            Some(raw_text.clone()),
                        ));
                    }
                },
                raw_text,
            ),
            res_type,
        ));
    }

    pub fn parse_bool_expr(&self, toks: &Vec<Token>) -> Result<Ast, ToyError> {
        if toks.len() == 1 {
            if toks[0].tok_type() == "BoolLit" {
                return Ok(Ast::BoolLit(match toks[0] {
                    Token::BoolLit(b) => b,
                    _ => unreachable!(),
                }));
            }
            if toks[0].tok_type() == "VarRef" {
                return self.parse_var_ref(&toks[0]);
            }
        }
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        let (best_idx, _, best_tok) = self.find_top_val(toks)?;
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, _) = self.parse_expr(&left.to_vec())?;
        let (r_node, _) = self.parse_expr(&right.to_vec())?;
        return Ok(Ast::InfixExpr(
            Box::new(l_node),
            Box::new(r_node),
            match best_tok {
                Token::LessThan => InfixOp::LessThan,
                Token::GreaterThan => InfixOp::GreaterThan,
                Token::LessThanEqt => InfixOp::LessThanEqt,
                Token::GreaterThanEqt => InfixOp::GreaterThanEqt,
                Token::And => InfixOp::And,
                Token::Or => InfixOp::Or,
                Token::Equals => InfixOp::Equals,
                Token::NotEquals => InfixOp::NotEquals,
                _ => {
                    return Err(ToyError::new(
                        ToyErrorType::InvalidInfixOperation,
                        Some(raw_text.clone()),
                    ));
                }
            },
            raw_text,
        ));
    }
    pub fn parse_str_expr(&self, toks: &Vec<Token>) -> Result<Ast, ToyError> {
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        if toks.len() == 1 {
            if toks[0].tok_type() == "StringLit" {
                return Ok(Ast::StringLit(
                    match toks[0].clone() {
                        Token::StringLit(b) => b,
                        _ => unreachable!(),
                    },
                    raw_text,
                ));
            }
            if toks[0].tok_type() == "VarRef" {
                return self.parse_var_ref(&toks[0]);
            }
        }
        //Only supported infix expression for strings is +
        let (best_idx, _, best_tok) = self.find_top_val(toks)?;
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, _) = self.parse_expr(&left.to_vec())?;
        let (r_node, _) = self.parse_expr(&right.to_vec())?;
        return Ok(Ast::InfixExpr(
            Box::new(l_node),
            Box::new(r_node),
            match best_tok {
                Token::Plus => InfixOp::Plus,
                _ => unreachable!(),
            },
            raw_text,
        ));
    }
    pub fn parse_empty_expr(&self, toks: &Vec<Token>) -> Result<(Ast, TypeTok), ToyError> {
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        if toks.is_empty() {
            return Err(ToyError::new(ToyErrorType::ExpectedExpression, None));
        }

        if toks[0].tok_type() != "LParen" {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                Some(raw_text.clone()),
            ));
        }

        let mut depth = 0;
        let mut end_idx = None;

        for (i, t) in toks.iter().enumerate() {
            match t.tok_type().as_str() {
                "LParen" => depth += 1,
                "RParen" => {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let end_idx = match end_idx.clone() {
            Some(i) => i,
            None => {
                return Err(ToyError::new(
                    ToyErrorType::UnclosedDelimiter,
                    Some(raw_text.clone()),
                ));
            }
        };

        let inner_toks = &toks[1..end_idx];
        let (inner_node, tok) = self.parse_expr(&inner_toks.to_vec())?;

        Ok((Ast::EmptyExpr(Box::new(inner_node), raw_text), tok))
    }
    pub fn parse_arr_lit(&self, toks: &Vec<Token>) -> Result<(Ast, TypeTok), ToyError> {
        let mut arr_toks: Vec<Token> = Vec::new();
        let mut depth = 0;
        for t in toks[1..].iter() {
            if t.tok_type() == "LBrack" {
                depth += 1;
            } else if t.tok_type() == "RBrack" {
                if depth == 0 {
                    break;
                } else {
                    depth -= 1;
                }
            }
            arr_toks.push(t.clone());
        }

        let mut arr_elems: Vec<Vec<Token>> = Vec::new();
        let mut current: Vec<Token> = Vec::new();
        let mut bracket_nest = 0;
        let mut brace_nest = 0;
        let mut paren_nest = 0;
        for t in arr_toks {
            match t.tok_type().as_str() {
                "LBrack" => bracket_nest += 1,
                "RBrack" => bracket_nest -= 1,
                "LBrace" => brace_nest += 1,
                "RBrace" => brace_nest -= 1,
                "LParen" => paren_nest += 1,
                "RParen" => paren_nest -= 1,
                _ => {}
            }

            if t.tok_type() == "Comma" && bracket_nest == 0 && brace_nest == 0 && paren_nest == 0 {
                arr_elems.push(current.clone());
                current.clear();
            } else {
                current.push(t);
            }
        }
        if !current.is_empty() {
            arr_elems.push(current);
        }

        // parse subexpressions
        let mut arr_types: Vec<TypeTok> = Vec::new();
        let mut arr_vals: Vec<Ast> = Vec::new();
        for elem in arr_elems {
            let (elem_ast, elem_type) = self.parse_expr(&elem)?;
            arr_vals.push(elem_ast);
            arr_types.push(elem_type);
        }

        let all_types_same = arr_types.windows(2).all(|w| w[0] == w[1]);
        let mut arr_type = TypeTok::Any;
        if all_types_same {
            arr_type = match arr_types[0].clone() {
                TypeTok::Int => TypeTok::IntArr(1),
                TypeTok::Bool => TypeTok::BoolArr(1),
                TypeTok::Float => TypeTok::FloatArr(1),
                TypeTok::Str => TypeTok::StrArr(1),
                TypeTok::Any => TypeTok::AnyArr(1),
                TypeTok::IntArr(n) => TypeTok::IntArr(n + 1),
                TypeTok::BoolArr(n) => TypeTok::BoolArr(n + 1),
                TypeTok::FloatArr(n) => TypeTok::FloatArr(n + 1),
                TypeTok::StrArr(n) => TypeTok::StrArr(n + 1),
                TypeTok::AnyArr(n) => TypeTok::AnyArr(n + 1),
                TypeTok::Struct(kv) => TypeTok::StructArr(kv, 1),
                TypeTok::StructArr(kv, n) => TypeTok::StructArr(kv, n + 1),
                other => other,
            };
        }

        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        Ok((Ast::ArrLit(arr_type.clone(), arr_vals, raw_text), arr_type))
    }
    pub fn parse_struct_def(
        &self,
        toks: &Vec<Token>,
        name: String,
    ) -> Result<(Ast, TypeTok), ToyError> {
        // Manually split the tokens between the braces at top-level commas,
        // so nested struct/array literals aren't split incorrectly.
        let inner = &toks[2..toks.len() - 1]; // tokens inside the `{ ... }`
        let mut unprocessed_kv: Vec<&[Token]> = Vec::new();
        let mut start = 0usize;
        let mut depth = 0i32;
        for (i, t) in inner.iter().enumerate() {
            match t.tok_type().as_str() {
                "LBrace" | "LBrack" | "LParen" => depth += 1,
                "RBrace" | "RBrack" | "RParen" => depth -= 1,
                "Comma" => {
                    if depth == 0 {
                        unprocessed_kv.push(&inner[start..i]);
                        start = i + 1;
                    }
                }
                _ => {}
            }
        }
        if start < inner.len() {
            unprocessed_kv.push(&inner[start..inner.len()]);
        }

        let mut processed_kv: BTreeMap<String, (Ast, TypeTok)> = BTreeMap::new();
        for kv in unprocessed_kv {
            let raw_text = toks
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<String>>()
                .join(" ");
            if kv.len() < 3 {
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructField,
                    Some(raw_text.clone()),
                ));
            }
            if kv[1].tok_type() != "Colon" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructField,
                    Some(raw_text.clone()),
                ));
            }
            let key = match kv[0].clone() {
                Token::VarRef(v) => *v,
                _ => {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedStructField,
                        Some(raw_text.clone()),
                    ));
                }
            };
            // kv[2..] are the tokens for the value (may be nested)
            let (value, value_type) = self.parse_expr(&kv[2..kv.len()].to_vec())?;
            let correct_type = match self.lookup_var_type(&name).unwrap().clone() {
                TypeTok::Struct(f) => *(f.get(&key).unwrap()).clone(),
                _ => {
                    return Err(ToyError::new(
                        ToyErrorType::VariableNotAStruct,
                        Some(raw_text.clone()),
                    ));
                }
            };
            if value_type != correct_type {
                return Err(ToyError::new(
                    ToyErrorType::TypeMismatch,
                    Some(raw_text.clone()),
                ));
            }
            processed_kv.insert(key, (value, value_type));
        }
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        Ok((
            Ast::StructLit(Box::new(name.clone()), Box::new(processed_kv), raw_text),
            self.lookup_var_type(&name).unwrap(),
        ))
    }

    pub fn parse_expr(&self, toks: &Vec<Token>) -> Result<(Ast, TypeTok), ToyError> {
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        //guard clause for not expressions - seems hacky but works
        if toks[0].tok_type() == "Not" {
            let (to_be_negated_val, to_be_negated_type) =
                self.parse_expr(&toks[1..toks.len()].to_vec())?;
            if to_be_negated_type != TypeTok::Bool {
                return Err(ToyError::new(
                    ToyErrorType::ExpressionNotBoolean,
                    Some(raw_text.clone()),
                ));
            }
            return Ok((Ast::Not(Box::new(to_be_negated_val)), TypeTok::Bool));
        }
        if toks.len() >= 3 {
            let mut i = 0usize;
            let mut new_toks: Vec<Token> = Vec::new();
            let mut found = false;
            while i < toks.len() {
                if i + 2 < toks.len()
                    && toks[i].tok_type() == "VarRef"
                    && toks[i + 1].tok_type() == "Dot"
                    && toks[i + 2].tok_type() == "VarRef"
                {
                    // collect keys
                    if let Token::VarRef(name) = toks[i].clone() {
                        let s_name = *name;
                        let mut keys: Vec<String> = Vec::new();
                        i += 1; // move to dot
                        while i + 1 < toks.len()
                            && toks[i].tok_type() == "Dot"
                            && toks[i + 1].tok_type() == "VarRef"
                        {
                            if let Token::VarRef(k) = toks[i + 1].clone() {
                                keys.push(*k);
                            }
                            i += 2;
                        }
                        new_toks.push(Token::StructRef(Box::new(s_name), keys));
                        found = true;
                        continue;
                    } else {
                        // should be unreachable
                        new_toks.push(toks[i].clone());
                        i += 1;
                        continue;
                    }
                }
                new_toks.push(toks[i].clone());
                i += 1;
            }
            if found {
                return self.parse_expr(&new_toks);
            }
        }

        //guard clause for single tokens
        if toks.len() == 1 {
            //struct ref (a.x or a.x.y)
            if toks[0].tok_type() == "StructRef" {
                let (s_name, keys) = match toks[0].clone() {
                    Token::StructRef(sn, k) => (*sn, k.clone()),
                    _ => unreachable!(),
                };

                let mut current_type: TypeTok = self.lookup_var_type(&s_name).unwrap();

                for key in &keys {
                    match current_type {
                        TypeTok::Struct(m) => {
                            current_type = if m.get(key).is_some() {
                                *m.get(key).unwrap().clone()
                            } else {
                                return Err(ToyError::new(
                                    ToyErrorType::KeyNotOnStruct,
                                    Some(raw_text.clone()),
                                ));
                            };
                        }
                        _ => {
                            return Err(ToyError::new(
                                ToyErrorType::VariableNotAStruct,
                                Some(raw_text.clone()),
                            ));
                        }
                    }
                }

                return Ok((
                    Ast::StructRef(Box::new(s_name), keys, raw_text.clone()),
                    current_type,
                ));
            }
            if toks[0].tok_type() == "IntLit" {
                return Ok((Ast::IntLit(toks[0].get_val().unwrap()), TypeTok::Int));
            }
            if toks[0].tok_type() == "FloatLit" {
                let val = match toks[0] {
                    Token::FloatLit(f) => f,
                    _ => unreachable!(),
                };
                return Ok((Ast::FloatLit(val), TypeTok::Float));
            }
            if toks[0].tok_type() == "StrLit" {
                let val = match toks[0].clone() {
                    Token::StringLit(s) => s,
                    _ => unreachable!(),
                };
                return Ok((Ast::StringLit(val, raw_text.clone()), TypeTok::Str));
            }
            if toks[0].tok_type() == "BoolLit" {
                let val = match toks[0].clone() {
                    Token::BoolLit(b) => b,
                    _ => unreachable!(),
                };
                return Ok((Ast::BoolLit(val), TypeTok::Bool));
            }
            if toks[0].tok_type() == "VarRef" {
                debug!(targets: ["parser_verbose"], "in var ref");
                let s = match toks[0].clone() {
                    Token::VarRef(name) => *name,
                    _ => unreachable!(),
                };
                let var_ref_type = self.lookup_var_type(&s);
                if var_ref_type.is_none() {
                    return Err(ToyError::new(
                        ToyErrorType::TypeHintNeeded,
                        Some(raw_text.clone()),
                    ));
                }
                return Ok((self.parse_var_ref(&toks[0])?, var_ref_type.unwrap().clone()));
            }
        }
        //guard clause for function calls
        if toks.first().unwrap().tok_type() == "VarRef" && toks[1].tok_type() == "LParen" {
            let mut depth = 0;
            let mut func_call_end = None;

            for (i, t) in toks.iter().enumerate().skip(1) {
                match t.tok_type().as_str() {
                    "LParen" => depth += 1,
                    "RParen" => {
                        depth -= 1;
                        if depth == 0 {
                            func_call_end = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(end_idx) = func_call_end {
                if end_idx == toks.len() - 1 {
                    return self.parse_func_call(toks);
                }
            }
        }
        //guard calls for empty expressions
        if toks.first().unwrap().tok_type() == "LParen"
            && toks.last().unwrap().tok_type() == "RParen"
        {
            let mut depth = 0;
            let mut first_paren_closes_at = None;

            for (i, t) in toks.iter().enumerate() {
                match t.tok_type().as_str() {
                    "LParen" => depth += 1,
                    "RParen" => {
                        depth -= 1;
                        if depth == 0 {
                            first_paren_closes_at = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if first_paren_closes_at == Some(toks.len() - 1) {
                let (inner, inner_type) = self.parse_expr(&toks[1..toks.len() - 1].to_vec())?;
                let to_ret_ast = Ast::EmptyExpr(Box::new(inner), raw_text.clone());
                return Ok((to_ret_ast, inner_type));
            }
        }
        //arr ref like arr[0]
        if toks.first().unwrap().tok_type() == "VarRef" && toks[1].tok_type() == "LBrack" {
            let name = match toks[0].clone() {
                Token::VarRef(a) => *a,
                _ => unreachable!(),
            };

            let mut i = 2;
            let mut idx_exprs: Vec<Ast> = Vec::new();
            let mut arr_ref_end = 0;

            while i < toks.len() {
                // find closing bracket
                let mut bracket_depth = 1;
                let mut j = i;
                while j < toks.len() && bracket_depth > 0 {
                    if toks[j].tok_type() == "LBrack" {
                        bracket_depth += 1;
                    } else if toks[j].tok_type() == "RBrack" {
                        bracket_depth -= 1;
                    }
                    j += 1;
                }

                if bracket_depth != 0 {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        Some(raw_text.clone()),
                    ));
                }

                let inner_toks = &toks[i..j - 1];
                let (idx_expr, _) = self.parse_num_expr(&inner_toks.to_vec())?;
                idx_exprs.push(idx_expr);
                arr_ref_end = j;

                if j >= toks.len() || toks[j].tok_type() != "LBrack" {
                    break;
                }

                i = j + 1;
            }

            let arr_type = match self.lookup_var_type(&name) {
                Some(t) => t,
                None => {
                    return Err(ToyError::new(
                        ToyErrorType::UndefinedVariable,
                        Some(raw_text.clone()),
                    ));
                }
            };

            let mut item_type = arr_type.clone();
            for _ in &idx_exprs {
                item_type = match item_type {
                    TypeTok::IntArr(1) => TypeTok::Int,
                    TypeTok::StrArr(1) => TypeTok::Str,
                    TypeTok::BoolArr(1) => TypeTok::Bool,
                    TypeTok::FloatArr(1) => TypeTok::Float,
                    TypeTok::AnyArr(1) => TypeTok::Any,
                    TypeTok::IntArr(n) => TypeTok::IntArr(n - 1),
                    TypeTok::StrArr(n) => TypeTok::StrArr(n - 1),
                    TypeTok::BoolArr(n) => TypeTok::BoolArr(n - 1),
                    TypeTok::FloatArr(n) => TypeTok::FloatArr(n - 1),
                    TypeTok::AnyArr(n) => TypeTok::AnyArr(n - 1),
                    TypeTok::StructArr(kv, 1) => TypeTok::Struct(kv),
                    TypeTok::StructArr(kv, n) => TypeTok::StructArr(kv, n - 1),
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::ArrayTypeInvalid,
                            Some(raw_text.clone()),
                        ));
                    } //if this error is triggered, something has gone very wrong
                };
            }

            let arr_ref_ast = Ast::ArrRef(Box::new(name.clone()), idx_exprs.clone(), raw_text.clone());

            if arr_ref_end < toks.len() {
                let remaining_toks = &toks[arr_ref_end..];
                if !remaining_toks.is_empty() {
                    let op_tok = &remaining_toks[0];

                    // Handle struct field access on array element: arr[i].field
                    if *op_tok == Token::Dot && remaining_toks.len() >= 2 {
                        let is_method_call = remaining_toks.len() >= 3
                            && remaining_toks[1].tok_type() == "VarRef"
                            && remaining_toks[2].tok_type() == "LParen";

                        if !is_method_call {
                            let mut keys: Vec<String> = Vec::new();
                            let mut current_type = item_type.clone();
                            let mut i = 1;

                            while i < remaining_toks.len() {
                                if remaining_toks[i].tok_type() == "VarRef" {
                                    let field_name = match &remaining_toks[i] {
                                        Token::VarRef(n) => *n.clone(),
                                        _ => unreachable!(),
                                    };

                                    if let TypeTok::Struct(fields) = current_type {
                                        if let Some(field_type) = fields.get(&field_name) {
                                            current_type = *field_type.clone();
                                            keys.push(field_name);
                                        } else {
                                            return Err(ToyError::new(
                                                ToyErrorType::KeyNotOnStruct,
                                                Some(raw_text.clone()),
                                            ));
                                        }
                                    } else {
                                        return Err(ToyError::new(
                                            ToyErrorType::VariableNotAStruct,
                                            Some(raw_text.clone()),
                                        ));
                                    }
                                    i += 1;
                                } else {
                                    break;
                                }

                                if i < remaining_toks.len() {
                                    if remaining_toks[i].tok_type() == "Dot" {
                                        i += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }

                            let arr_struct_ref = Ast::ArrStructRef(
                                Box::new(name.clone()),
                                idx_exprs.clone(),
                                keys,
                                raw_text.clone(),
                            );

                            if i < remaining_toks.len() {
                                let next_op_tok = &remaining_toks[i];
                                let rest_toks = &remaining_toks[i + 1..];
                                let (rhs_ast, _rhs_type) = self.parse_expr(&rest_toks.to_vec())?;

                                let op = match next_op_tok {
                                    Token::Plus => InfixOp::Plus,
                                    Token::Minus => InfixOp::Minus,
                                    Token::Multiply => InfixOp::Multiply,
                                    Token::Divide => InfixOp::Divide,
                                    Token::Modulo => InfixOp::Modulo,
                                    Token::LessThan => InfixOp::LessThan,
                                    Token::LessThanEqt => InfixOp::LessThanEqt,
                                    Token::GreaterThan => InfixOp::GreaterThan,
                                    Token::GreaterThanEqt => InfixOp::GreaterThanEqt,
                                    Token::Equals => InfixOp::Equals,
                                    Token::NotEquals => InfixOp::NotEquals,
                                    Token::And => InfixOp::And,
                                    Token::Or => InfixOp::Or,
                                    _ => {
                                        return Err(ToyError::new(
                                            ToyErrorType::InvalidInfixOperation,
                                            Some(raw_text.clone()),
                                        ));
                                    }
                                };

                                return Ok((
                                    Ast::InfixExpr(
                                        Box::new(arr_struct_ref),
                                        Box::new(rhs_ast),
                                        op.clone(),
                                        raw_text.clone(),
                                    ),
                                    match op {
                                        InfixOp::Plus
                                        | InfixOp::Minus
                                        | InfixOp::Multiply
                                        | InfixOp::Divide
                                        | InfixOp::Modulo => current_type,
                                        _ => TypeTok::Bool,
                                    },
                                ));
                            } else {
                                return Ok((arr_struct_ref, current_type));
                            }
                        }
                    }

                    // Handle method call on array element: arr[i].method(...)
                    if *op_tok == Token::Dot && remaining_toks.len() >= 3 {
                        if remaining_toks[1].tok_type() == "VarRef"
                            && remaining_toks[2].tok_type() == "LParen"
                        {
                            // This is a method call on array element
                            // Convert arr[i].method(...) to method(arr[i], ...)
                            let method_name = match &remaining_toks[1] {
                                Token::VarRef(n) => n.clone(),
                                _ => unreachable!(),
                            };

                            // Build new tokens: method_name(arr[i], args...)
                            let mut new_toks: Vec<Token> = Vec::new();
                            new_toks.push(Token::VarRef(method_name));
                            new_toks.push(Token::LParen);

                            // Create a token for the array reference
                            // We need to pass the original array ref tokens
                            let arr_ref_toks = &toks[0..arr_ref_end];
                            new_toks.extend_from_slice(arr_ref_toks);

                            // Add comma if there are more arguments
                            let args_start = 3; // after Dot, VarRef, LParen
                            if args_start < remaining_toks.len()
                                && remaining_toks[args_start].tok_type() != "RParen"
                            {
                                new_toks.push(Token::Comma);
                            }

                            // Add the rest of the arguments (everything after LParen)
                            new_toks.extend_from_slice(&remaining_toks[args_start..]);

                            // Parse the transformed function call
                            return self.parse_expr(&new_toks);
                        }
                    }

                    let right_toks = &remaining_toks[1..];
                    let (right_ast, right_type) = self.parse_expr(&right_toks.to_vec())?;

                    let op = match op_tok {
                        Token::Plus => InfixOp::Plus,
                        Token::Minus => InfixOp::Minus,
                        Token::Multiply => InfixOp::Multiply,
                        Token::Divide => InfixOp::Divide,
                        Token::Modulo => InfixOp::Modulo,
                        Token::LessThan => InfixOp::LessThan,
                        Token::LessThanEqt => InfixOp::LessThanEqt,
                        Token::GreaterThan => InfixOp::GreaterThan,
                        Token::GreaterThanEqt => InfixOp::GreaterThanEqt,
                        Token::Equals => InfixOp::Equals,
                        Token::NotEquals => InfixOp::NotEquals,
                        Token::And => InfixOp::And,
                        Token::Or => InfixOp::Or,
                        _ => {
                            return Err(ToyError::new(
                                ToyErrorType::InvalidInfixOperation,
                                Some(raw_text.clone()),
                            ));
                        }
                    };

                    let result_type = match op {
                        InfixOp::Plus
                        | InfixOp::Minus
                        | InfixOp::Multiply
                        | InfixOp::Divide
                        | InfixOp::Modulo => {
                            if item_type == TypeTok::Float || right_type == TypeTok::Float {
                                TypeTok::Float
                            } else if item_type == TypeTok::Str || right_type == TypeTok::Str {
                                TypeTok::Str
                            } else {
                                TypeTok::Int
                            }
                        }
                        _ => TypeTok::Bool,
                    };

                    return Ok((
                        Ast::InfixExpr(
                            Box::new(arr_ref_ast),
                            Box::new(right_ast),
                            op,
                            raw_text.clone(),
                        ),
                        result_type,
                    ));
                }
            }

            return Ok((arr_ref_ast, item_type));
        }

        //Arr literals
        if toks.first().unwrap().tok_type() == "LBrack" {
            return self.parse_arr_lit(toks);
        }
        //Struct literal
        if toks.first().unwrap().tok_type() == "VarRef" && toks[1].tok_type() == "LBrace" {
            let name = match toks[0].clone() {
                Token::VarRef(n) => *n,
                _ => unreachable!(),
            };
            let mut i = 2_usize;
            let mut struct_dec_exprs: Vec<Ast> = Vec::new();
            let mut struct_dec_types: Vec<TypeTok> = Vec::new();
            while i < toks.len() {
                let mut bracket_depth = 1_i32;
                let mut j = i;
                while j < toks.len() && bracket_depth > 0 {
                    if toks[j].tok_type() == "LBrace" {
                        bracket_depth += 1;
                    } else if toks[j].tok_type() == "RBrace" {
                        bracket_depth -= 1;
                    }
                    j += 1;
                }
                if bracket_depth != 0 {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        Some(raw_text.clone()),
                    ));
                }
                let inner_toks = &toks[i - 2..j];
                let (inner_expr, t) = self.parse_struct_def(&inner_toks.to_vec(), name.clone())?;
                struct_dec_types.push(t);
                struct_dec_exprs.push(inner_expr);
                if j >= toks.len() || toks[j].tok_type() == "LBrace" {
                    break;
                }
                i = j + 1;
            }
            return Ok((struct_dec_exprs[0].clone(), struct_dec_types[0].clone()));
        }

        let (best_idx, _, best_val) = self.find_top_val(toks)?;
        debug!(targets: ["parser", "parser_verbose"], best_val.clone());
        debug!(targets: ["parser", "parser_verbose"], toks.clone());
        return match best_val {
            Token::IntLit(_) | Token::Plus | Token::FloatLit(_) => {
                let left = &toks[0..best_idx];
                let (_, left_type) = self.parse_expr(&left.to_vec())?;

                let res = match left_type {
                    TypeTok::Str => (self.parse_str_expr(toks)?, TypeTok::Str),
                    TypeTok::Int | TypeTok::Float => self.parse_num_expr(toks)?,
                    TypeTok::Bool => (self.parse_bool_expr(toks)?, TypeTok::Bool),
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::InvalidOperationOnGivenType,
                            Some(raw_text.clone()),
                        ));
                    } //like the second of three times I check this
                };
                return Ok(res);
            }
            Token::Minus | Token::Divide | Token::Multiply | Token::Modulo => {
                self.parse_num_expr(toks)
            }
            Token::BoolLit(_)
            | Token::LessThan
            | Token::LessThanEqt
            | Token::GreaterThan
            | Token::GreaterThanEqt
            | Token::Equals
            | Token::NotEquals
            | Token::And
            | Token::Or => Ok((self.parse_bool_expr(toks)?, TypeTok::Bool)),
            Token::StringLit(_) => Ok((self.parse_str_expr(toks)?, TypeTok::Str)),
            Token::LParen | Token::RBrace => self.parse_empty_expr(toks),
            _ => {
                return Err(ToyError::new(
                    ToyErrorType::ExpectedExpression,
                    Some(raw_text.clone()),
                ));
            }
        };
    }
}
