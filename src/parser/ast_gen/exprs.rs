use super::AstGenerator;
use crate::debug;
use crate::errors::{ToyError, ToyErrorType};
use crate::parser::ast::{Ast, InfixOp};
use crate::token::{SpannedToken, Token, TypeTok};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

impl AstGenerator {
    pub fn parse_num_expr(&self, toks: &Vec<SpannedToken>) -> Result<(Ast, TypeTok), ToyError> {
        let cumulate_span = AstGenerator::total_span(toks.clone());
        if toks.len() == 1 {
            if toks[0].tok.tok_type() == "IntLit" {
                return Ok((
                    Ast::IntLit(toks[0].tok.get_val().unwrap(), cumulate_span),
                    TypeTok::Int,
                ));
            }
            if toks[0].tok.tok_type() == "VarRef" {
                let name = match &toks[0].tok {
                    Token::VarRef(n) => n,
                    _ => unreachable!(),
                };

                let ty = self
                    .lookup_var_type(name)
                    .ok_or_else(|| ToyError::new(ToyErrorType::TypeHintNeeded, cumulate_span))?;
                return Ok((self.parse_var_ref(&toks[0])?, ty));
            }
            if toks[0].tok.tok_type() == "FloatLit" {
                let val = match toks[0].tok {
                    Token::FloatLit(f) => f,
                    _ => unreachable!(),
                };
                return Ok((Ast::FloatLit(val, cumulate_span), TypeTok::Float));
            }
        }
        if toks.len() == 0 {
            return Err(ToyError::new(
                ToyErrorType::ExpectedExpression,
                cumulate_span,
            ));
        }

        let (best_idx, _, best_tok) = self.find_top_val(toks)?;

        if best_idx == 0 && best_tok.tok == Token::Minus {
            let right = &toks[1..];
            let (r_node, r_type) = self.parse_expr(&right.to_vec())?;
            let lhs = if r_type == TypeTok::Float {
                Ast::FloatLit(OrderedFloat(0.0), cumulate_span.clone())
            } else {
                Ast::IntLit(0, cumulate_span.clone()) //this looks wrong
            };

            return Ok((
                Ast::InfixExpr(
                    Box::new(lhs),
                    Box::new(r_node),
                    InfixOp::Minus,
                    cumulate_span,
                ),
                r_type,
            ));
        }

        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, l_type) = self
            .parse_expr(&left.to_vec())
            .map_err(|e| e.with_context(cumulate_span.clone()))?;
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
                match best_tok.tok {
                    Token::Plus => InfixOp::Plus,
                    Token::Minus => InfixOp::Minus,
                    Token::Multiply => InfixOp::Multiply,
                    Token::Divide => InfixOp::Divide,
                    Token::Modulo => InfixOp::Modulo,
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::InvalidInfixOperation,
                            cumulate_span,
                        ));
                    }
                },
                cumulate_span,
            ),
            res_type,
        ));
    }

    pub fn parse_bool_expr(&self, toks: &Vec<SpannedToken>) -> Result<Ast, ToyError> {
        let cumulative_span = AstGenerator::total_span(toks.clone());
        if toks.len() == 1 {
            if toks[0].tok.tok_type() == "BoolLit" {
                return Ok(Ast::BoolLit(
                    match toks[0].tok {
                        Token::BoolLit(b) => b,
                        _ => unreachable!(),
                    },
                    cumulative_span,
                ));
            }
            if toks[0].tok.tok_type() == "VarRef" {
                return self.parse_var_ref(&toks[0]);
            }
        }

        let (best_idx, _, best_tok) = self.find_top_val(toks)?;
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, _) = self.parse_expr(&left.to_vec())?;
        let (r_node, _) = self.parse_expr(&right.to_vec())?;
        return Ok(Ast::InfixExpr(
            Box::new(l_node),
            Box::new(r_node),
            match best_tok.tok {
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
                        cumulative_span,
                    ));
                }
            },
            cumulative_span,
        ));
    }
    pub fn parse_str_expr(&self, toks: &Vec<SpannedToken>) -> Result<Ast, ToyError> {
        let cumulative_span = AstGenerator::total_span(toks.clone());
        if toks.len() == 1 {
            if toks[0].tok.tok_type() == "StringLit" {
                return Ok(Ast::StringLit(
                    match toks[0].clone().tok {
                        Token::StringLit(b) => b,
                        _ => unreachable!(),
                    },
                    cumulative_span,
                ));
            }
            if toks[0].tok.tok_type() == "VarRef" {
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
            match best_tok.tok {
                Token::Plus => InfixOp::Plus,
                _ => unreachable!(),
            },
            cumulative_span,
        ));
    }
    pub fn parse_empty_expr(&self, toks: &Vec<SpannedToken>) -> Result<(Ast, TypeTok), ToyError> {
        let cumulative_span = AstGenerator::total_span(toks.clone());
        if toks.is_empty() {
            return Err(ToyError::new(
                ToyErrorType::ExpectedExpression,
                cumulative_span,
            ));
        }

        if toks[0].tok.tok_type() != "LParen" {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                cumulative_span,
            ));
        }

        let mut depth = 0;
        let mut end_idx = None;

        for (i, t) in toks.iter().enumerate() {
            match t.tok.tok_type().as_str() {
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
                    cumulative_span,
                ));
            }
        };

        let inner_toks = &toks[1..end_idx];
        let (inner_node, tok) = self.parse_expr(&inner_toks.to_vec())?;

        return Ok((Ast::EmptyExpr(Box::new(inner_node), cumulative_span), tok));
    }
    pub fn parse_arr_lit(&self, toks: &Vec<SpannedToken>) -> Result<(Ast, TypeTok), ToyError> {
        let mut arr_toks: Vec<SpannedToken> = Vec::new();
        let cumulative_span = AstGenerator::total_span(toks.clone());
        let mut depth = 0;
        for t in toks[1..].iter() {
            if t.tok.tok_type() == "LBrack" {
                depth += 1;
            } else if t.tok.tok_type() == "RBrack" {
                if depth == 0 {
                    break;
                } else {
                    depth -= 1;
                }
            }
            arr_toks.push(t.clone());
        }

        let mut arr_elems: Vec<Vec<SpannedToken>> = Vec::new();
        let mut current: Vec<SpannedToken> = Vec::new();
        let mut bracket_nest = 0;
        let mut brace_nest = 0;
        let mut paren_nest = 0;
        for t in arr_toks {
            match t.tok.tok_type().as_str() {
                "LBrack" => bracket_nest += 1,
                "RBrack" => bracket_nest -= 1,
                "LBrace" => brace_nest += 1,
                "RBrace" => brace_nest -= 1,
                "LParen" => paren_nest += 1,
                "RParen" => paren_nest -= 1,
                _ => {}
            }

            if t.tok.tok_type() == "Comma"
                && bracket_nest == 0
                && brace_nest == 0
                && paren_nest == 0
            {
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

        if arr_types.is_empty() {
            return Ok((
                Ast::ArrLit(TypeTok::Any, arr_vals, cumulative_span),
                TypeTok::Any,
            ));
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

        return Ok((
            Ast::ArrLit(arr_type.clone(), arr_vals, cumulative_span),
            arr_type,
        ));
    }
    pub fn parse_struct_def(
        &self,
        toks: &Vec<SpannedToken>,
        name: String,
    ) -> Result<(Ast, TypeTok), ToyError> {
        let cumulative_span = AstGenerator::total_span(toks.clone());
        // Manually split the tokens between the braces at top-level commas,
        // so nested struct/array literals aren't split incorrectly.
        let inner = if toks.len() >= 2
            && toks[0].tok.tok_type() == "VarRef"
            && toks[1].tok.tok_type() == "LBrace"
        {
            &toks[2..toks.len() - 1]
        } else if toks.len() >= 2 && toks[0].tok.tok_type() == "LBrace" {
            &toks[1..toks.len() - 1]
        } else {
            toks
        };
        let mut unprocessed_kv: Vec<&[SpannedToken]> = Vec::new();
        let mut start = 0usize;
        let mut depth = 0i32;
        for (i, t) in inner.iter().enumerate() {
            match t.tok.tok_type().as_str() {
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
            if kv.len() < 3 {
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructField,
                    cumulative_span,
                ));
            }
            if kv[1].tok.tok_type() != "Colon" {
                return Err(ToyError::new(
                    ToyErrorType::MalformedStructField,
                    cumulative_span,
                ));
            }
            let key = match kv[0].clone().tok {
                Token::VarRef(v) => *v,
                _ => {
                    return Err(ToyError::new(
                        ToyErrorType::MalformedStructField,
                        cumulative_span,
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
                        cumulative_span,
                    ));
                }
            };
            if value_type != correct_type {
                return Err(ToyError::new(ToyErrorType::TypeMismatch, cumulative_span));
            }
            processed_kv.insert(key, (value, value_type));
        }

        Ok((
            Ast::StructLit(
                Box::new(name.clone()),
                Box::new(processed_kv),
                cumulative_span,
            ),
            self.lookup_var_type(&name).unwrap(),
        ))
    }

    pub fn parse_expr(&self, toks: &Vec<SpannedToken>) -> Result<(Ast, TypeTok), ToyError> {
        let cumulative_span = AstGenerator::total_span(toks.clone());
        if toks.is_empty() {
            return Err(ToyError::new(
                ToyErrorType::ExpectedExpression,
                cumulative_span,
            ));
        }

        //guard clause for not expressions
        if toks[0].tok.tok_type() == "Not" {
            let (to_be_negated_val, to_be_negated_type) =
                self.parse_expr(&toks[1..toks.len()].to_vec())?;
            if to_be_negated_type != TypeTok::Bool {
                return Err(ToyError::new(
                    ToyErrorType::ExpressionNotBoolean,
                    cumulative_span,
                ));
            }
            return Ok((
                Ast::Not(Box::new(to_be_negated_val), cumulative_span),
                TypeTok::Bool,
            ));
        }

        //guard clause for single tokens
        if toks.len() == 1 {
            if toks[0].tok.tok_type() == "IntLit" {
                return Ok((
                    Ast::IntLit(toks[0].tok.get_val().unwrap(), cumulative_span),
                    TypeTok::Int,
                ));
            }
            if toks[0].tok.tok_type() == "FloatLit" {
                let val = match toks[0].tok {
                    Token::FloatLit(f) => f,
                    _ => unreachable!(),
                };
                return Ok((Ast::FloatLit(val, cumulative_span), TypeTok::Float));
            }
            if toks[0].tok.tok_type() == "StrLit" {
                let val = match toks[0].clone().tok {
                    Token::StringLit(s) => s,
                    _ => unreachable!(),
                };
                return Ok((Ast::StringLit(val, cumulative_span), TypeTok::Str));
            }
            if toks[0].tok.tok_type() == "BoolLit" {
                let val = match toks[0].clone().tok {
                    Token::BoolLit(b) => b,
                    _ => unreachable!(),
                };
                return Ok((Ast::BoolLit(val, cumulative_span), TypeTok::Bool));
            }
            if toks[0].tok.tok_type() == "VarRef" {
                debug!(targets: ["parser_verbose"], "in var ref");
                let s = match toks[0].clone().tok {
                    Token::VarRef(name) => *name,
                    _ => unreachable!(),
                };
                let var_ref_type = self.lookup_var_type(&s);
                if var_ref_type.is_none() {
                    return Err(ToyError::new(ToyErrorType::TypeHintNeeded, cumulative_span));
                }
                return Ok((self.parse_var_ref(&toks[0])?, var_ref_type.unwrap().clone()));
            }
        }

        //guard clause for function calls
        if toks.first().unwrap().tok.tok_type() == "VarRef" && toks[1].tok.tok_type() == "LParen" {
            let mut depth = 0;
            let mut func_call_end = None;

            for (i, t) in toks.iter().enumerate().skip(1) {
                match t.tok.tok_type().as_str() {
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

        //guard calls for empty expressions (parens)
        if toks.first().unwrap().tok.tok_type() == "LParen"
            && toks.last().unwrap().tok.tok_type() == "RParen"
        {
            let mut depth = 0;
            let mut first_paren_closes_at = None;

            for (i, t) in toks.iter().enumerate() {
                match t.tok.tok_type().as_str() {
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
                let to_ret_ast = Ast::EmptyExpr(Box::new(inner), cumulative_span);
                return Ok((to_ret_ast, inner_type));
            }
        }

        //Arr literals
        if toks.first().unwrap().tok.tok_type() == "LBrack" {
            // Check if it's an array literal or index access on something else?
            // If it starts with LBrack, it must be ArrLit because IndexAccess requires LHS.
            return self.parse_arr_lit(toks);
        }

        //Struct literal
        if toks.first().unwrap().tok.tok_type() == "VarRef" && toks[1].tok.tok_type() == "LBrace" {
            let name = match toks[0].clone().tok {
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
                    if toks[j].tok.tok_type() == "LBrace" {
                        bracket_depth += 1;
                    } else if toks[j].tok.tok_type() == "RBrace" {
                        bracket_depth -= 1;
                    }
                    j += 1;
                }
                if bracket_depth != 0 {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        cumulative_span,
                    ));
                }
                let inner_toks = &toks[i - 2..j];
                let (inner_expr, t) = self.parse_struct_def(&inner_toks.to_vec(), name.clone())?;
                struct_dec_types.push(t);
                struct_dec_exprs.push(inner_expr);
                if j >= toks.len() || toks[j].tok.tok_type() == "LBrace" {
                    break;
                }
                i = j + 1;
            }
            if i == toks.len() {
                return Ok((struct_dec_exprs[0].clone(), struct_dec_types[0].clone()));
            }
        }

        let (best_idx, _, best_val) = self.find_top_val(toks)?;
        debug!(targets: ["parser", "parser_verbose"], best_val.clone());
        debug!(targets: ["parser", "parser_verbose"], toks.clone());

        match best_val.tok {
            Token::Dot => {
                let left = &toks[0..best_idx];
                let right = &toks[best_idx + 1..toks.len()];

                if left.len() == 1 {
                    if let Some(name) = left[0].get_var_name() {
                        if let Some(full_module_name) = self.imports.get(&*name) {
                            if right.len() >= 3
                                && right[0].tok.tok_type() == "VarRef"
                                && right[1].tok.tok_type() == "LParen"
                                && right.last().unwrap().tok.tok_type() == "RParen"
                            {
                                let func_name = match &right[0].tok {
                                    Token::VarRef(n) => *n.clone(),
                                    _ => unreachable!(),
                                };
                                let prefix = full_module_name.replace(".", "::");
                                let full_name = format!("{}::{}", prefix, func_name);

                                let args_toks = &right[2..right.len() - 1];
                                let mut args = Vec::new();
                                let mut arg_types = Vec::new();
                                let mut current_arg_toks = Vec::new();
                                let mut depth = 0;
                                for t in args_toks {
                                    if t.tok.tok_type() == "Comma" && depth == 0 {
                                        let (arg_ast, arg_type) =
                                            self.parse_expr(&current_arg_toks)?;
                                        args.push(arg_ast);
                                        arg_types.push(arg_type);
                                        current_arg_toks.clear();
                                    } else {
                                        if t.tok.tok_type() == "LParen"
                                            || t.tok.tok_type() == "LBrace"
                                            || t.tok.tok_type() == "LBrack"
                                        {
                                            depth += 1;
                                        } else if t.tok.tok_type() == "RParen"
                                            || t.tok.tok_type() == "RBrace"
                                            || t.tok.tok_type() == "RBrack"
                                        {
                                            depth -= 1;
                                        }
                                        current_arg_toks.push(t.clone());
                                    }
                                }
                                if !current_arg_toks.is_empty() {
                                    let (arg_ast, arg_type) = self.parse_expr(&current_arg_toks)?;
                                    args.push(arg_ast);
                                    arg_types.push(arg_type);
                                }

                                let mut mangled_full_name = full_name.clone();
                                for t in &arg_types {
                                    mangled_full_name = format!(
                                        "{}_{}",
                                        mangled_full_name,
                                        t.type_str().to_lowercase()
                                    );
                                }

                                let mut final_name = mangled_full_name.clone();
                                let mut ret_type =
                                    self.func_return_type_map.get(&final_name).cloned();

                                if ret_type.is_none() {
                                    if let Some(rt) = self.func_return_type_map.get(&full_name) {
                                        final_name = full_name;
                                        ret_type = Some(rt.clone());
                                    }
                                }

                                let ret_type = ret_type.unwrap_or(TypeTok::Void);

                                return Ok((
                                    Ast::FuncCall(Box::new(final_name), args, cumulative_span),
                                    ret_type,
                                ));
                            }
                        }
                    }
                }

                let (left_ast, left_type) = self.parse_expr(&left.to_vec())?;

                // Check for Method Call: name(...)
                if right.len() >= 3
                    && right[0].tok.tok_type() == "VarRef"
                    && right[1].tok.tok_type() == "LParen"
                    && right.last().unwrap().tok.tok_type() == "RParen"
                {
                    let method_name = match &right[0].tok {
                        Token::VarRef(n) => *n.clone(),
                        _ => unreachable!(),
                    };

                    let fields = match &left_type {
                        TypeTok::Struct(f) => f,
                        _ => {
                            return Err(ToyError::new(
                                ToyErrorType::VariableNotAStruct,
                                cumulative_span,
                            ));
                        }
                    };

                    let struct_name = self.struct_type_to_name.get(fields).ok_or_else(|| {
                        ToyError::new(ToyErrorType::VariableNotAStruct, cumulative_span.clone())
                    })?;

                    let mangled_name = format!("{}:::{}", struct_name, method_name);

                    let args_toks = &right[2..right.len() - 1];
                    let mut args = Vec::new();
                    let mut arg_types = Vec::new();
                    let mut current_arg_toks = Vec::new();
                    let mut depth = 0;
                    for t in args_toks {
                        if t.tok.tok_type() == "Comma" && depth == 0 {
                            let (arg_ast, arg_type) = self.parse_expr(&current_arg_toks)?;
                            args.push(arg_ast);
                            arg_types.push(arg_type);
                            current_arg_toks.clear();
                        } else {
                            if t.tok.tok_type() == "LParen"
                                || t.tok.tok_type() == "LBrace"
                                || t.tok.tok_type() == "LBrack"
                            {
                                depth += 1;
                            } else if t.tok.tok_type() == "RParen"
                                || t.tok.tok_type() == "RBrace"
                                || t.tok.tok_type() == "RBrack"
                            {
                                depth -= 1;
                            }
                            current_arg_toks.push(t.clone());
                        }
                    }
                    if !current_arg_toks.is_empty() {
                        let (arg_ast, arg_type) = self.parse_expr(&current_arg_toks)?;
                        args.push(arg_ast);
                        arg_types.push(arg_type);
                    }

                    args.insert(0, left_ast);
                    arg_types.insert(0, left_type);

                    let mut final_mangled_name = mangled_name.clone();
                    for t in &arg_types {
                        final_mangled_name =
                            format!("{}_{}", final_mangled_name, t.type_str().to_lowercase());
                    }

                    let ret_type = self
                        .func_return_type_map
                        .get(&final_mangled_name)
                        .ok_or_else(|| {
                            ToyError::new(ToyErrorType::UndefinedFunction, cumulative_span.clone())
                        })?
                        .clone();

                    return Ok((
                        Ast::FuncCall(Box::new(final_mangled_name), args, cumulative_span),
                        ret_type,
                    ));
                }

                if right.len() != 1 {
                    return Err(ToyError::new(
                        ToyErrorType::ExpectedIdentifier,
                        cumulative_span,
                    ));
                }
                let member_name = match &right[0].tok {
                    Token::VarRef(n) => *n.clone(),
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::ExpectedIdentifier,
                            cumulative_span,
                        ));
                    }
                };

                let member_type = match left_type {
                    TypeTok::Struct(fields) => match fields.get(&member_name) {
                        Some(t) => *t.clone(),
                        None => {
                            return Err(ToyError::new(
                                ToyErrorType::KeyNotOnStruct,
                                cumulative_span,
                            ));
                        }
                    },
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::VariableNotAStruct,
                            cumulative_span,
                        ));
                    }
                };

                Ok((
                    Ast::MemberAccess(Box::new(left_ast), member_name, cumulative_span),
                    member_type,
                ))
            }
            Token::LBrack => {
                let left = &toks[0..best_idx];
                if toks.last().unwrap().tok.tok_type() != "RBrack" {
                    return Err(ToyError::new(
                        ToyErrorType::UnclosedDelimiter,
                        cumulative_span,
                    ));
                }
                let index_toks = &toks[best_idx + 1..toks.len() - 1];

                let (left_ast, left_type) = self.parse_expr(&left.to_vec())?;
                let (index_ast, index_type) = self.parse_num_expr(&index_toks.to_vec())?;

                if index_type != TypeTok::Int {
                    return Err(ToyError::new(ToyErrorType::TypeMismatch, cumulative_span));
                }

                let elem_type = match left_type {
                    TypeTok::IntArr(n) => {
                        if n == 1 {
                            TypeTok::Int
                        } else {
                            TypeTok::IntArr(n - 1)
                        }
                    }
                    TypeTok::StrArr(n) => {
                        if n == 1 {
                            TypeTok::Str
                        } else {
                            TypeTok::StrArr(n - 1)
                        }
                    }
                    TypeTok::BoolArr(n) => {
                        if n == 1 {
                            TypeTok::Bool
                        } else {
                            TypeTok::BoolArr(n - 1)
                        }
                    }
                    TypeTok::FloatArr(n) => {
                        if n == 1 {
                            TypeTok::Float
                        } else {
                            TypeTok::FloatArr(n - 1)
                        }
                    }
                    TypeTok::AnyArr(n) => {
                        if n == 1 {
                            TypeTok::Any
                        } else {
                            TypeTok::AnyArr(n - 1)
                        }
                    }
                    TypeTok::StructArr(kv, n) => {
                        if n == 1 {
                            TypeTok::Struct(kv)
                        } else {
                            TypeTok::StructArr(kv, n - 1)
                        }
                    }
                    _ => {
                        return Err(ToyError::new(
                            ToyErrorType::ArrayTypeInvalid,
                            cumulative_span,
                        ));
                    }
                };

                Ok((
                    Ast::IndexAccess(Box::new(left_ast), Box::new(index_ast), cumulative_span),
                    elem_type,
                ))
            }
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
                            cumulative_span,
                        ));
                    }
                };
                return Ok(res);
            }
            Token::VarRef(_) => {
                let right = &toks[best_idx + 1..toks.len()];
                if !right.is_empty() && right[0].tok.tok_type() == "LBrace" {
                    if right.last().unwrap().tok.tok_type() != "RBrace" {
                        return Err(ToyError::new(
                            ToyErrorType::UnclosedDelimiter,
                            cumulative_span,
                        ));
                    }
                    let inner_toks = &right[1..right.len() - 1];
                    let name = match &toks[best_idx].tok {
                        Token::VarRef(n) => *n.clone(),
                        _ => unreachable!(),
                    };
                    let (ast, ty) = self.parse_struct_def(&inner_toks.to_vec(), name)?;
                    Ok((ast, ty))
                } else if best_idx == 0 && right.is_empty() {
                    let name = match &toks[0].tok {
                        Token::VarRef(n) => n,
                        _ => unreachable!(),
                    };
                    let ty = self.lookup_var_type(name).ok_or_else(|| {
                        ToyError::new(ToyErrorType::TypeHintNeeded, cumulative_span)
                    })?;
                    Ok((self.parse_var_ref(&toks[0])?, ty))
                } else {
                    Err(ToyError::new(
                        ToyErrorType::ExpectedExpression,
                        cumulative_span,
                    ))
                }
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
                    cumulative_span,
                ));
            }
        }
    }
}
