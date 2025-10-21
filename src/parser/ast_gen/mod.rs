use crate::debug;
use crate::parser::ast::Ast;
use crate::parser::toy_box::TBox;
use crate::token::Token;
use crate::token::TypeTok;
use std::collections::HashMap;
mod exprs;
pub struct AstGenerator {
    boxes: Vec<TBox>,
    nodes: Vec<Ast>,
    bp: usize,
    p_table: HashMap<String, u32>,
    // Scopes are stacked to avoid variable name collisions
    var_type_scopes: Vec<HashMap<String, TypeTok>>,
    func_param_type_map: HashMap<String, Vec<TypeTok>>,
    func_return_type_map: HashMap<String, TypeTok>,
}

impl AstGenerator {
    pub fn new() -> AstGenerator {
        let b_vec: Vec<TBox> = Vec::new();
        let n_vec: Vec<Ast> = Vec::new();
        let mut map: HashMap<String, u32> = HashMap::new();
        map.insert(Token::LParen.tok_type(), 10000);
        map.insert(Token::RParen.tok_type(), 10000);
        map.insert(Token::VarRef(Box::new("".to_string())).tok_type(), 100);
        map.insert(Token::IntLit(0).tok_type(), 100);
        map.insert(Token::BoolLit(true).tok_type(), 100);
        map.insert(Token::StringLit(Box::new("".to_string())).tok_type(), 100);

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

        let mut var_type_scopes = Vec::new();
        var_type_scopes.push(HashMap::new());

        let mut fptm: HashMap<String, Vec<TypeTok>> = HashMap::new();
        fptm.insert("print".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("println".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("len".to_string(), [TypeTok::Str].to_vec());
        fptm.insert("str".to_string(), [TypeTok::Any].to_vec());
        let mut frtm: HashMap<String, TypeTok> = HashMap::new();
        frtm.insert("print".to_string(), TypeTok::Void);
        frtm.insert("println".to_string(), TypeTok::Void);
        frtm.insert("len".to_string(), TypeTok::Int);
        frtm.insert("str".to_string(), TypeTok::Str);

        return AstGenerator {
            boxes: b_vec,
            nodes: n_vec,
            bp: 0_usize,
            p_table: map,
            var_type_scopes,
            func_param_type_map: fptm,
            func_return_type_map: frtm,
        };
    }

    fn push_scope(&mut self) {
        self.var_type_scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.var_type_scopes.len() > 1 {
            self.var_type_scopes.pop();
        } else {
            panic!("[ERROR] Cannot pop global scope");
        }
    }

    fn lookup_var_type(&self, name: &str) -> Option<TypeTok> {
        for scope in self.var_type_scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    ///Puts var the innermost scope
    fn insert_var_type(&mut self, name: String, ty: TypeTok) {
        if let Some(current_scope) = self.var_type_scopes.last_mut() {
            current_scope.insert(name, ty);
        }
    }

    fn find_top_val(&self, toks: &Vec<Token>) -> (usize, u32, Token) {
        let mut best_idx = 0_usize;
        let mut best_val: u32 = 100_000_000;
        let mut best_tok: Token = Token::IntLit(0);

        let mut depth = 0;

        for (i, t) in toks.iter().enumerate() {
            match t.tok_type().as_str() {
                "LParen" => {
                    depth += 1;
                    continue;
                }
                "RParen" => {
                    if depth == 0 {
                        panic!("[ERROR] Unmatched RParen at index {}, toks: {:?}", i, toks);
                    }
                    depth -= 1;
                    continue;
                }
                _ => {}
            }

            if depth != 0 {
                continue;
            }

            let maybe_val = self.p_table.get(&t.tok_type());
            if maybe_val.is_none() {
                panic!("[ERROR] Unknown symbol, got {}", t);
            }

            let val = *maybe_val.unwrap();
            if val < best_val {
                best_val = val;
                best_idx = i;
                best_tok = t.clone();
            }
        }

        if depth != 0 {
            panic!("[ERROR] Unmatched LParen in expression");
        }

        (best_idx, best_val, best_tok)
    }

    fn parse_func_call(&self, toks: &Vec<Token>) -> (Ast, TypeTok) {
        if toks[0].tok_type() != "VarRef" {
            panic!("[ERROR] Expected Var(Func) ref, got {}", toks[0]);
        }
        if toks[1].tok_type() != "LParen" {
            panic!("[ERROR] Expected \"(\", got {}", toks[1]);
        }
        if toks.last().unwrap().tok_type() != "RParen" {
            panic!("[ERROR] Expected \")\", got {}", toks.last().unwrap());
        }
        let name = match toks[0].clone() {
            Token::VarRef(n) => *n,
            _ => unreachable!(),
        };

        let unprocessed_params: Vec<&[Token]> = toks[2..toks.len() - 1]
            .split(|t| *t == Token::Comma)
            .collect();
        let mut processed_params: Vec<(Ast, TypeTok)> = Vec::new();
        for p in unprocessed_params {
            processed_params.push(self.parse_expr(&p.to_vec()));
        }
        let types = self.func_param_type_map.get(&name);
        if types.is_none() {
            panic!("[ERROR] Function {} is undefined", name);
        }

        let types = types.unwrap();
        for (i, (_, type_tok)) in processed_params.iter().enumerate() {
            if type_tok != &types[i] && types[i] != TypeTok::Any {
                panic!(
                    "[ERROR] Mismatched types at index {}, expected {:?}, got {:?}",
                    i, types[i], type_tok
                );
            }
        }
        let vals: Vec<Ast> = processed_params
            .iter()
            .filter_map(|ast| {
                let (a, _) = ast;
                Some(a.clone())
            })
            .collect();
        return (
            Ast::FuncCall(Box::new(name.clone()), vals),
            self.func_return_type_map
                .get(&name.clone())
                .unwrap()
                .clone(),
        );
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
        self.insert_var_type(name_str.clone(), ret_var_type.clone());
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
        let (cond, body, alt) = match stmt {
            TBox::IfStmt(c, b, a) => (c, b, a),
            _ => panic!("[ERROR] Expected IfStmt, got {}", stmt),
        };

        let b_cond = self.parse_bool_expr(&cond);

        self.push_scope();
        let mut stmt_vec: Vec<Ast> = Vec::new();
        for stmt in body {
            debug!(targets: ["parser_verbose"], stmt);
            stmt_vec.push(self.parse_stmt(stmt, false));
        }
        self.pop_scope();

        let mut else_val: Option<Vec<Ast>> = None;
        if alt.is_some() {
            self.push_scope();
            let mut else_vec: Vec<Ast> = Vec::new();
            for stmt in alt.unwrap() {
                else_vec.push(self.parse_stmt(stmt, false));
            }
            self.pop_scope();
            else_val = Some(else_vec);
        }
        let if_stmt = Ast::IfStmt(Box::new(b_cond), stmt_vec, else_val);

        if should_eat {
            self.eat();
        }

        return if_stmt;
    }

    fn parse_func_dec(&mut self, stmt: TBox, should_eat: bool) -> Ast {
        let (name_tok, params, return_type, box_boxy) = match stmt {
            TBox::FuncDec(n, p, r, b) => (n, p, r, b),
            _ => panic!("[ERROR] Expected FuncDec, got {}", stmt),
        };
        let name = match name_tok {
            Token::VarName(n) => *n,
            _ => panic!(
                "[ERROR] Expected function (variable) name, got {}",
                name_tok
            ),
        };

        let mut ast_params: Vec<Ast> = Vec::new();
        let mut param_types: Vec<TypeTok> = Vec::new();

        for param in params {
            let (param_name, param_type) = match param {
                TBox::FuncParam(name, type_tok) => {
                    let n = match name {
                        Token::VarRef(var) => *var,
                        _ => panic!("[ERROR] Expected variable reference, got {}", name),
                    };
                    (n, type_tok)
                }
                _ => panic!("[ERROR] Expected function parameter, got {}", param),
            };

            ast_params.push(Ast::FuncParam(
                Box::new(param_name.clone()),
                param_type.clone(),
            ));
            param_types.push(param_type.clone());
        }

        self.func_param_type_map.insert(name.clone(), param_types);
        self.func_return_type_map
            .insert(name.clone(), return_type.clone());

        self.push_scope();
        for param_ast in &ast_params {
            if let Ast::FuncParam(param_name, param_type) = param_ast {
                self.insert_var_type((**param_name).clone(), param_type.clone());
            }
        }

        let mut body: Vec<Ast> = Vec::new();
        for stmt in box_boxy {
            body.push(self.parse_stmt(stmt, false))
        }

        self.pop_scope();

        if should_eat {
            self.eat();
        }

        return Ast::FuncDec(Box::new(name), ast_params, return_type, body);
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
            TBox::IfStmt(_, _, _) => {
                return self.parse_if_stmt(val, should_eat);
            }
            TBox::FuncDec(_, _, _, _) => return self.parse_func_dec(val, should_eat),
            TBox::Return(val) => {
                let expr = match *val {
                    TBox::Expr(ref v) => v,
                    _ => panic!("[ERROR] Return must return an expression, got {}", val),
                };

                let (res, _) = self.parse_expr(expr);
                return Ast::Return(Box::new(res));
            }

            TBox::While(expr, body) => {
                let parsed_expr = self.parse_bool_expr(&expr);
                let mut parsed_body: Vec<Ast> = Vec::new();
                for stmt in body {
                    parsed_body.push(self.parse_stmt(stmt, false))
                }
                if should_eat{
                    self.eat();
                }
                return Ast::WhileStmt(Box::new(parsed_expr), parsed_body);
            }
            TBox::Continue => Ast::Continue,
            TBox::Break => Ast::Break,
            _ => todo!("Unimplemented statement"),
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
