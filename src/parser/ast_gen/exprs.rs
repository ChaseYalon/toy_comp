use super::AstGenerator;
use crate::debug;
use crate::parser::ast::{Ast, InfixOp};
use crate::token::{Token, TypeTok};

impl AstGenerator {
    pub fn parse_int_expr(&self, toks: &Vec<Token>) -> Ast {
        if toks.len() == 1 {
            if toks[0].tok_type() == "IntLit" {
                return Ast::IntLit(toks[0].get_val().unwrap());
            }
            if toks[0].tok_type() == "VarRef" {
                return self.parse_var_ref(&toks[0]);
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
                    _ => panic!("this is impossible"),
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
    pub fn parse_expr(&self, toks: &Vec<Token>) -> (Ast, TypeTok) {
        if toks.len() == 1 {
            if toks[0].tok_type() == "IntLit" {
                return (Ast::IntLit(toks[0].get_val().unwrap()), TypeTok::Int);
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
                    _ => panic!("[ERROR] Expected variable name, got {}", toks[0]),
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
        if toks.first().unwrap().tok_type() == "VarRef" && toks[1].tok_type() == "LParen" {
            return self.parse_func_call(toks);
        }
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

        let (best_idx, _, best_val) = self.find_top_val(toks);
        debug!(targets: ["parser", "parser_verbose"], best_val.clone());
        debug!(targets: ["parser", "parser_verbose"], toks.clone());
        return match best_val {
            Token::IntLit(_) | Token::Plus => {
                //This is a screwy way to do it but I cant think of a better way
                if toks[best_idx - 1].tok_type() == "StringLit" {
                    (self.parse_str_expr(toks), TypeTok::Str)
                } else if toks[best_idx - 1].tok_type() == "VarRef" {
                    let var = match toks[best_idx - 1].clone() {
                        Token::VarRef(v) => *v,
                        _ => unreachable!(),
                    };
                    let t = self.lookup_var_type(&var);
                    if t.is_none() {
                        panic!("[ERROR] Variable {} is undefined", var);
                    }
                    let ty = t.unwrap();
                    if ty == TypeTok::Str {
                        return (self.parse_str_expr(toks), TypeTok::Str);
                    } else if ty == TypeTok::Int {
                        return (self.parse_int_expr(toks), TypeTok::Int);
                    } else {
                        return (self.parse_bool_expr(toks), TypeTok::Bool);
                    }
                } else {
                    (self.parse_str_expr(toks), TypeTok::Int)
                }
            }
            Token::Minus | Token::Divide | Token::Multiply | Token::Modulo => {
                (self.parse_int_expr(toks), TypeTok::Int)
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
