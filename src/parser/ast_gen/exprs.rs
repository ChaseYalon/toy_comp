use super::AstGenerator;
use crate::debug;
use crate::parser::ast::{Ast, InfixOp};
use crate::token::{Token, TypeTok};
use std::collections::HashMap;
impl AstGenerator {
    pub fn parse_num_expr(&self, toks: &Vec<Token>) -> Ast {
        if toks.len() == 1 {
            if toks[0].tok_type() == "IntLit" {
                return Ast::IntLit(toks[0].get_val().unwrap());
            }
            if toks[0].tok_type() == "VarRef" {
                return self.parse_var_ref(&toks[0]);
            }
            if toks[0].tok_type() == "FloatLit" {
                let val = match toks[0] {
                    Token::FloatLit(f) => f,
                    _ => unreachable!(),
                };
                return Ast::FloatLit(val);
            }
        }
        if toks.len() == 0 {
            panic!("[ERROR] Empty Expression");
        }
        let (best_idx, _, best_tok) = self.find_top_val(toks);
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, _) = self.parse_expr(&left.to_vec());
        let (r_node, _) = self.parse_expr(&right.to_vec());
        return Ast::InfixExpr(
            Box::new(l_node),
            Box::new(r_node),
            match best_tok {
                Token::Plus => InfixOp::Plus,
                Token::Minus => InfixOp::Minus,
                Token::Multiply => InfixOp::Multiply,
                Token::Divide => InfixOp::Divide,
                Token::Modulo => InfixOp::Modulo,
                _ => panic!("[ERROR] WTF happened here, got operator {}", best_tok),
            },
        );
    }

    pub fn parse_bool_expr(&self, toks: &Vec<Token>) -> Ast {
        if toks.len() == 1 {
            if toks[0].tok_type() == "BoolLit" {
                return Ast::BoolLit(match toks[0] {
                    Token::BoolLit(b) => b,
                    _ => unreachable!(),
                });
            }
            if toks[0].tok_type() == "VarRef" {
                return self.parse_var_ref(&toks[0]);
            }
        }
        let (best_idx, _, best_tok) = self.find_top_val(toks);
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, _) = self.parse_expr(&left.to_vec());
        let (r_node, _) = self.parse_expr(&right.to_vec());
        return Ast::InfixExpr(
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
                _ => panic!("[ERROR] Wtf happened here (bool)"),
            },
        );
    }
    pub fn parse_str_expr(&self, toks: &Vec<Token>) -> Ast {
        if toks.len() == 1 {
            if toks[0].tok_type() == "StringLit" {
                return Ast::StringLit(match toks[0].clone() {
                    Token::StringLit(b) => b,
                    _ => unreachable!(),
                });
            }
            if toks[0].tok_type() == "VarRef" {
                return self.parse_var_ref(&toks[0]);
            }
        }
        //Only supported infix expression for strings is +
        let (best_idx, _, best_tok) = self.find_top_val(toks);
        let left = &toks[0..best_idx];
        let right = &toks[best_idx + 1..toks.len()];

        let (l_node, _) = self.parse_expr(&left.to_vec());
        let (r_node, _) = self.parse_expr(&right.to_vec());
        return Ast::InfixExpr(
            Box::new(l_node),
            Box::new(r_node),
            match best_tok {
                Token::Plus => InfixOp::Plus,
                _ => unreachable!(),
            },
        );
    }
    pub fn parse_empty_expr(&self, toks: &Vec<Token>) -> (Ast, TypeTok) {
        if toks.is_empty() {
            panic!("[ERROR] No tokens provided for empty expression");
        }

        if toks[0].tok_type() != "LParen" {
            panic!("[ERROR] Expecting LParen, got {}", toks[0].clone());
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

        let end_idx = end_idx.expect("[ERROR] No matching RParen found");

        let inner_toks = &toks[1..end_idx];
        let (inner_node, tok) = self.parse_expr(&inner_toks.to_vec());

        (Ast::EmptyExpr(Box::new(inner_node)), tok)
    }
    pub fn parse_arr_lit(&self, toks: &Vec<Token>) -> (Ast, TypeTok) {
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
        let mut nest = 0;
        for t in arr_toks {
            if t.tok_type() == "LBrack" {
                nest += 1;
            } else if t.tok_type() == "RBrack" {
                nest -= 1;
            }

            if t.tok_type() == "Comma" && nest == 0 {
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
            let (elem_ast, elem_type) = self.parse_expr(&elem);
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
                other => other,
            };
        }

        (Ast::ArrLit(arr_type.clone(), arr_vals), arr_type)
    }
    pub fn parse_struct_def(&self, toks: &Vec<Token>, name: String) -> (Ast, TypeTok) {
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

        let mut processed_kv: HashMap<String, (Ast, TypeTok)> = HashMap::new();
        for kv in unprocessed_kv {
            if kv.len() < 3 {
                panic!("[ERROR] Malformed struct kv: {:?}", kv);
            }
            if kv[1].tok_type() != "Colon" {
                panic!(
                    "[ERROR] Expected Colon between name and value, got {}",
                    kv[1].clone()
                );
            }
            let key = match kv[0].clone() {
                Token::VarRef(v) => *v,
                _ => panic!("[ERROR] Expected name, got {}", kv[0]),
            };
            // kv[2..] are the tokens for the value (may be nested)
            let (value, value_type) = self.parse_expr(&kv[2..kv.len()].to_vec());
            let correct_type = match self.lookup_var_type(&name).unwrap().clone() {
                TypeTok::Struct(f) => *(f.get(&key).unwrap()).clone(),
                _ => panic!(
                    "[ERROR] Variable {} is not a struct, it is a {:?}",
                    name,
                    self.lookup_var_type(&name).unwrap()
                ),
            };
            if value_type != correct_type {
                panic!("[ERROR] Expected {:?}, got {:?}", correct_type, value_type);
            }
            processed_kv.insert(key, (value, value_type));
        }
        (
            Ast::StructLit(Box::new(name.clone()), Box::new(processed_kv)),
            self.lookup_var_type(&name).unwrap(),
        )
    }

    pub fn parse_expr(&self, toks: &Vec<Token>) -> (Ast, TypeTok) {
        //guard clause for single tokens
        if toks.len() == 1 {
            //struct ref (a.x or a.x.y)
            if toks[0].tok_type() == "StructRef" {
                let (s_name, keys) = match toks[0].clone() {
                    Token::StructRef(sn, k) => (*sn, k.clone()),
                    _ => unreachable!(),
                };

                let mut current_type = self.lookup_var_type(&s_name).unwrap();

                for key in &keys {
                    match current_type {
                        TypeTok::Struct(m) => {
                            current_type = *m
                                .get(key)
                                .unwrap_or_else(|| {
                                    panic!("[ERROR] Field '{}' not found in struct", key)
                                })
                                .clone();
                        }
                        _ => panic!(
                            "[ERROR] Cannot access field '{}' on non-struct type {:?}",
                            key, current_type
                        ),
                    }
                }

                return (Ast::StructRef(Box::new(s_name), keys), current_type);
            }
            if toks[0].tok_type() == "IntLit" {
                return (Ast::IntLit(toks[0].get_val().unwrap()), TypeTok::Int);
            }
            if toks[0].tok_type() == "FloatLit" {
                let val = match toks[0] {
                    Token::FloatLit(f) => f,
                    _ => unreachable!(),
                };
                return (Ast::FloatLit(val), TypeTok::Float);
            }
            if toks[0].tok_type() == "StrLit" {
                let val = match toks[0].clone() {
                    Token::StringLit(s) => s,
                    _ => unreachable!(),
                };
                return (Ast::StringLit(val), TypeTok::Str);
            }
            if toks[0].tok_type() == "BoolLit" {
                let val = match toks[0].clone() {
                    Token::BoolLit(b) => b,
                    _ => unreachable!(),
                };
                return (Ast::BoolLit(val), TypeTok::Bool);
            }
            if toks[0].tok_type() == "VarRef" {
                debug!(targets: ["parser_verbose"], "in var ref");
                let s = match toks[0].clone() {
                    Token::VarRef(name) => *name,
                    _ => unreachable!(),
                };
                let var_ref_type = self.lookup_var_type(&s);
                if var_ref_type.is_none() {
                    panic!(
                        "[ERROR] Could not figure out type of variable, {}",
                        &toks[0]
                    );
                }
                return (self.parse_var_ref(&toks[0]), var_ref_type.unwrap().clone());
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
                let (inner, inner_type) = self.parse_expr(&toks[1..toks.len() - 1].to_vec());
                let to_ret_ast = Ast::EmptyExpr(Box::new(inner));
                return (to_ret_ast, inner_type);
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
                    panic!("[ERROR] Unclosed brackets in array access for '{}'", name);
                }

                let inner_toks = &toks[i..j - 1];
                let idx_expr = self.parse_num_expr(&inner_toks.to_vec());
                idx_exprs.push(idx_expr);

                if j >= toks.len() || toks[j].tok_type() != "LBrack" {
                    break;
                }

                i = j + 1;
            }

            let arr_type = match self.lookup_var_type(&name) {
                Some(t) => t,
                None => panic!("[ERROR] Variable {} is undefined", name),
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
                    _ => panic!(
                        "[ERROR] {:?} is not an array type or dimension mismatch",
                        item_type
                    ),
                };
            }

            return (Ast::ArrRef(Box::new(name), idx_exprs), item_type);
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
                    panic!(
                        "[ERROR] Unclosed brace in struct literal dec, Got: {:?}",
                        toks
                    );
                }
                let inner_toks = &toks[i - 2..j];
                let (inner_expr, t) = self.parse_struct_def(&inner_toks.to_vec(), name.clone());
                struct_dec_types.push(t);
                struct_dec_exprs.push(inner_expr);
                if j >= toks.len() || toks[j].tok_type() == "LBrace" {
                    break;
                }
                i = j + 1;
            }
            return (struct_dec_exprs[0].clone(), struct_dec_types[0].clone());
        }

        let (best_idx, _, best_val) = self.find_top_val(toks);
        debug!(targets: ["parser", "parser_verbose"], best_val.clone());
        debug!(targets: ["parser", "parser_verbose"], toks.clone());
        return match best_val {
            Token::IntLit(_) | Token::Plus | Token::FloatLit(_) => {
                let left = &toks[0..best_idx];
                let (_, left_type) = self.parse_expr(&left.to_vec());

                // if either side has float, type promote
                let has_float = toks.iter().any(|t| t.tok_type() == "FloatLit");

                match left_type {
                    TypeTok::Str => (self.parse_str_expr(toks), TypeTok::Str),
                    TypeTok::Int if has_float => (self.parse_num_expr(toks), TypeTok::Float),
                    TypeTok::Int => (self.parse_num_expr(toks), TypeTok::Int),
                    TypeTok::Bool => (self.parse_bool_expr(toks), TypeTok::Bool),
                    TypeTok::Float => (self.parse_num_expr(toks), TypeTok::Float),
                    _ => panic!(
                        "[ERROR] Unsupported type for Plus operation: {:?}",
                        left_type
                    ),
                }
            }
            Token::Minus | Token::Divide | Token::Multiply | Token::Modulo => {
                // if there is any float should type promote to float
                let has_float = toks.iter().any(|t| t.tok_type() == "FloatLit");

                if has_float {
                    (self.parse_num_expr(toks), TypeTok::Float)
                } else {
                    (self.parse_num_expr(toks), TypeTok::Int)
                }
            }
            Token::BoolLit(_)
            | Token::LessThan
            | Token::LessThanEqt
            | Token::GreaterThan
            | Token::GreaterThanEqt
            | Token::Equals
            | Token::NotEquals
            | Token::And
            | Token::Or => (self.parse_bool_expr(toks), TypeTok::Bool),
            Token::StringLit(_) => (self.parse_str_expr(toks), TypeTok::Str),
            Token::LParen | Token::RBrace => self.parse_empty_expr(toks),
            _ => panic!("[ERROR] Unsupported type for expression, got {}", best_val),
        };
    }
}