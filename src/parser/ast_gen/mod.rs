use crate::debug;
use crate::parser::ast::Ast;
use crate::parser::ast::InfixOp;
use crate::parser::toy_box::TBox;
use crate::token::Token;
use crate::token::TypeTok;
use std::collections::HashMap;
pub struct AstGenerator {
    boxes: Vec<TBox>,
    nodes: Vec<Ast>,
    bp: usize,
    //Static lifetime to make red line go away
    p_table: HashMap<String, u32>,
    var_type_map: HashMap<String, TypeTok>,
}
impl AstGenerator {
    pub fn new() -> AstGenerator {
        let b_vec: Vec<TBox> = Vec::new();
        let n_vec: Vec<Ast> = Vec::new();
        let mut map: HashMap<String, u32> = HashMap::new();
        map.insert(Token::VarRef(Box::new("".to_string())).tok_type(), 100);
        map.insert(Token::IntLit(0).tok_type(), 100); //Zero is just an arbitrary value, it will be fine for all int literals
        map.insert(Token::BoolLit(true).tok_type(), 100);

        map.insert(Token::Multiply.tok_type(), 4);
        map.insert(Token::Divide.tok_type(), 4);
        map.insert(Token::Modulo.tok_type(), 4);

        map.insert(Token::Plus.tok_type(), 3);
        map.insert(Token::Minus.tok_type(), 3);

        map.insert(Token::LessThan.tok_type(), 2);
        map.insert(Token::LessThanEqt.tok_type(), 2);
        map.insert(Token::GreaterThan.tok_type(), 2);
        map.insert(Token::GreaterThanEqt.tok_type(), 2);
        map.insert(Token::Equals.tok_type(), 2);
        map.insert(Token::NotEquals.tok_type(), 2);

        map.insert(Token::And.tok_type(), 1);
        map.insert(Token::Or.tok_type(), 1);

        let v_type_map: HashMap<String, TypeTok> = HashMap::new();

        return AstGenerator {
            boxes: b_vec,
            nodes: n_vec,
            bp: 0_usize,
            p_table: map,
            var_type_map: v_type_map,
        };
    }
    fn find_top_val(&self, toks: &Vec<Token>) -> (usize, u32, Token) {
        let mut best_idx = 0_usize;
        let mut best_val: u32 = 100000000; //Practical infinity
        let mut best_tok: Token = Token::IntLit(0); //This will throw an error later if its val has not been changed
        for (i, t) in toks.iter().enumerate() {
            if t.tok_type() == "IntLit" {
                continue;
            }
            let maybe_val = self.p_table.get(&t.tok_type());

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
        return (best_idx, best_val, best_tok);
    }
    fn parse_int_expr(&self, toks: &Vec<Token>) -> Ast {
        if toks.len() == 1 {
            if toks[0].tok_type() == "IntLit" {
                return Ast::IntLit(toks[0].get_val().unwrap());
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
                Token::Plus => InfixOp::Plus,
                Token::Minus => InfixOp::Minus,
                Token::Multiply => InfixOp::Multiply,
                Token::Divide => InfixOp::Divide,
                Token::Modulo => InfixOp::Modulo,
                _ => panic!("[ERROR] WTF happened here, got opperator {}", best_tok),
            },
        );
    }
    fn parse_bool_expr(&self, toks: &Vec<Token>) -> Ast {
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
    fn parse_expr(&self, toks: &Vec<Token>) -> (Ast, TypeTok) {
        if toks.len() == 1 {
            if toks[0].tok_type() == "IntLit" {
                return (Ast::IntLit(toks[0].get_val().unwrap()), TypeTok::Int);
            }
            if toks[0].tok_type() == "VarRef" {
                debug!(targets: ["parser_verbose"], "in var ref");
                let s = match toks[0].clone() {
                    Token::VarRef(name) => *name,
                    _ => panic!("[ERROR] Expected variable name, got {}", toks[0]),
                };
                let var_ref_type = self.var_type_map.get(&s);
                if var_ref_type.is_none() {
                    panic!(
                        "[ERROR] Could not figure out type of variable, {}",
                        &toks[0]
                    );
                }
                return (self.parse_var_ref(&toks[0]), var_ref_type.unwrap().clone());
            }
        }
        let (_, _, best_val) = self.find_top_val(toks);
        debug!(targets: ["parser", "parser_verbose"], best_val.clone());
        debug!(targets: ["parser", "parser_verbose"], toks.clone());
        return match best_val {
            Token::IntLit(_)
            | Token::Plus
            | Token::Minus
            | Token::Divide
            | Token::Multiply
            | Token::Modulo => (self.parse_int_expr(toks), TypeTok::Int),
            Token::BoolLit(_)
            | Token::LessThan
            | Token::LessThanEqt
            | Token::GreaterThan
            | Token::GreaterThanEqt
            | Token::Equals
            | Token::NotEquals
            | Token::And
            | Token::Or => (self.parse_bool_expr(toks), TypeTok::Bool),
            _ => panic!("[ERROR] Unsupported type for expression, got {}", best_val),
        };
    }
    pub fn eat(&mut self) {
        self.bp += 1;
    }
    fn parse_var_dec(&mut self, name: &Token, val: &Vec<Token>, var_type: Option<TypeTok>) -> Ast {
        if name.tok_type() != "VarName" {
            panic!("[ERROR] Expected variable name, got {}", name);
        }
        let name_str = *name.get_var_name().unwrap();
        let (val_ast, val_type) = self.parse_expr(val);
        let ret_var_type: TypeTok;
        if var_type.is_some() {
            ret_var_type = var_type.unwrap();
        } else {
            ret_var_type = val_type;
        }
        let node = Ast::VarDec(
            Box::new(name_str.clone()),
            ret_var_type.clone(),
            Box::new(val_ast),
        );
        self.var_type_map
            .insert(name_str.clone(), ret_var_type.clone());
        return node;
    }
    fn parse_var_ref(&self, name: &Token) -> Ast {
        let name_s: String;
        match name {
            Token::VarRef(box_str) => name_s = *box_str.clone(),
            _ => panic!("[ERROR] Expected var_ref, got {}", name),
        }
        return Ast::VarRef(Box::new(name_s));
    }
    fn parse_if_stmt(&mut self, stmt: TBox, should_eat: bool) -> Ast {
        let (cond, body) = match stmt {
            TBox::IfStmt(c, b) => (c, b),
            _ => panic!("[ERROR] Expected IfStmt, got {}", stmt),
        };

        let b_cond = self.parse_bool_expr(&cond);
        let mut stmt_vec: Vec<Ast> = Vec::new();
        for stmt in body {
            debug!(targets: ["parser_verbose"], stmt);
            stmt_vec.push(self.parse_stmt(stmt, false));
        }
        let if_stmt = Ast::IfStmt(Box::new(b_cond), stmt_vec);
        
        if should_eat {
            self.eat();
        }
        
        return if_stmt;
    }

    fn parse_stmt(&mut self, val: TBox, should_eat: bool) -> Ast {
        debug!(targets: ["parser_verbose"], val);
        
        let node = match val {
            TBox::Expr(i) => {
                let (node, _) = self.parse_expr(&i);
                node
            }
            TBox::VarDec(name, var_type, v_val) => {
                self.parse_var_dec(&name, &v_val, var_type.clone())
            }
            TBox::VarRef(name) => {
                debug!(targets: ["parser_verbose"], name);
                self.parse_var_ref(&name)
            }
            TBox::VarReassign(var, val) => {
                let var_node = self.parse_var_ref(&var);
                let (val_node, _) = self.parse_expr(&val);
                Ast::VarReassign(
                    Box::new(match var_node {
                        Ast::VarRef(i) => i.to_string(),
                        _ => "".to_string(),
                    }),
                    Box::new(val_node),
                )
            }
            TBox::IfStmt(_, _) => {
                return self.parse_if_stmt(val, should_eat);
            }
        };
        
        if should_eat {
            self.eat();
        }
        
        node
    }
    pub fn generate(&mut self, boxes: Vec<TBox>) -> Vec<Ast> {
        self.boxes = boxes.clone();
        self.bp = 0_usize;
        debug!(targets: ["parser_verbose"], boxes);
        while self.bp < self.boxes.len() {
            let val = self.boxes[self.bp].clone();
            debug!(targets: ["parser_verbose"], val);
            let stmt = self.parse_stmt(val, true);
            self.nodes.push(stmt)
        }
        return self.nodes.clone();
    }
}

#[cfg(test)]
mod test;
