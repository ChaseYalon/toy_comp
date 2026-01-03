use ordered_float::OrderedFloat;

use crate::debug;
use crate::errors::{ToyError, ToyErrorType};
use crate::parser::ast::Ast;
use crate::parser::toy_box::TBox;
use crate::token::Token;
use crate::token::TypeTok;
use std::collections::{BTreeMap, HashMap, HashSet};
use crate::lexer::Lexer;
use crate::parser::boxer::Boxer;
use std::fs;

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
    // Maps struct field signature to struct name for method resolution
    struct_type_to_name: HashMap<BTreeMap<String, Box<TypeTok>>, String>,
    ///module name -> path so std.math maps to /std/math.toy (posix)
    imports: HashMap<String, String>,
    extern_funcs: HashSet<String>,
}

impl AstGenerator {
    pub fn new() -> AstGenerator {
        let b_vec: Vec<TBox> = Vec::new();
        let n_vec: Vec<Ast> = Vec::new();
        let mut map: HashMap<String, u32> = HashMap::new();
        map.insert(Token::LParen.tok_type(), 10000);
        map.insert(Token::RParen.tok_type(), 10000);
        map.insert(Token::LBrack.tok_type(), 80); // IndexAccess
        map.insert(Token::RBrack.tok_type(), 10000);
        map.insert(Token::Dot.tok_type(), 80); // MemberAccess

        map.insert(Token::StringLit(Box::new("".to_string())).tok_type(), 100);
        map.insert(Token::VarRef(Box::new("".to_string())).tok_type(), 100);
        map.insert(
            Token::StructRef(Box::new("".to_string()), Vec::new()).tok_type(),
            100,
        );
        map.insert(Token::IntLit(0).tok_type(), 100);
        map.insert(Token::BoolLit(true).tok_type(), 100);
        map.insert(Token::StringLit(Box::new("".to_string())).tok_type(), 100);
        map.insert(Token::FloatLit(OrderedFloat(0.0)).tok_type(), 100);

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
        map.insert(Token::Not.tok_type(), 1); //we must be careful with not, it can only bind to the right

        let mut var_type_scopes = Vec::new();
        var_type_scopes.push(HashMap::new());

        let mut fptm: HashMap<String, Vec<TypeTok>> = HashMap::new();
        fptm.insert("print".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("println".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("len".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("str".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("bool".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("int".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("float".to_string(), [TypeTok::Any].to_vec());
        fptm.insert("input".to_string(), [TypeTok::Str].to_vec());

        let mut frtm: HashMap<String, TypeTok> = HashMap::new();
        frtm.insert("print".to_string(), TypeTok::Void);
        frtm.insert("println".to_string(), TypeTok::Void);
        frtm.insert("len".to_string(), TypeTok::Int);
        frtm.insert("str".to_string(), TypeTok::Str);
        frtm.insert("bool".to_string(), TypeTok::Bool);
        frtm.insert("int".to_string(), TypeTok::Int);
        frtm.insert("float".to_string(), TypeTok::Float);
        frtm.insert("input".to_string(), TypeTok::Str);

        return AstGenerator {
            boxes: b_vec,
            nodes: n_vec,
            bp: 0_usize,
            p_table: map,
            var_type_scopes,
            func_param_type_map: fptm,
            func_return_type_map: frtm,
            struct_type_to_name: HashMap::new(),
            imports: HashMap::new(),
            extern_funcs: HashSet::new(),
        };
    }

    fn load_module(&mut self, name: &str) {
        let path = format!("{}.toy", name.replace(".", "/"));
        if let Ok(content) = fs::read_to_string(&path) {
             let mut l = Lexer::new();
             if let Ok(toks) = l.lex(content) {
                 let mut b = Boxer::new();
                 if let Ok(boxes) = b.box_toks(toks) {
                     let prefix = name.replace(".", "::");
                     for b in boxes {
                         match b {
                             TBox::ExternFuncDec(name_tok, _, ret_type, _) => {
                                 if let Some(n) = name_tok.get_var_name() {
                                     let full_name = format!("{}::{}", prefix, n);
                                     self.func_return_type_map.insert(full_name.clone(), ret_type);
                                     self.extern_funcs.insert(full_name);
                                 }
                             }
                             TBox::FuncDec(name_tok, _, ret_type, _, _) => {
                                 if let Some(n) = name_tok.get_var_name() {
                                     let full_name = format!("{}::{}", prefix, n);
                                     self.func_return_type_map.insert(full_name, ret_type);
                                 }
                             }
                             _ => {}
                         }
                     }
                 }
             }
        }
    }

    fn push_scope(&mut self) {
        self.var_type_scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) -> Result<(), ToyError> {
        if self.var_type_scopes.len() > 1 {
            self.var_type_scopes.pop();
        } else {
            return Err(ToyError::new(ToyErrorType::InternalParserFailure, None));
        }
        return Ok(());
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

    fn find_top_val(&self, toks: &Vec<Token>) -> Result<(usize, u32, Token), ToyError> {
        let mut best_idx = 0_usize;
        let mut best_val: u32 = 100_000_000;
        let mut best_tok: Token = Token::IntLit(0);

        let mut depth = 0;
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");

        for (i, t) in toks.iter().enumerate() {
            match t.tok_type().as_str() {
                "LParen" => {
                    depth += 1;
                    continue;
                }
                "RParen" => {
                    if depth == 0 {
                        return Err(ToyError::new(
                            ToyErrorType::UnclosedDelimiter,
                            Some(raw_text.clone()),
                        ));
                    }
                    depth -= 1;
                    continue;
                }
                "LBrace" => {
                    depth += 1;
                    continue;
                }
                "RBrace" => {
                    if depth == 0 {
                        return Err(ToyError::new(
                            ToyErrorType::UnclosedDelimiter,
                            Some(raw_text.clone()),
                        ));
                    }
                    depth -= 1;
                    continue;
                }
                "LBrack" => {
                    if depth == 0 && i > 0 {
                        let maybe_val = self.p_table.get(&t.tok_type());
                        if let Some(val) = maybe_val {
                            if *val <= best_val {
                                best_val = *val;
                                best_idx = i;
                                best_tok = t.clone();
                            }
                        }
                    }
                    depth += 1;
                    continue;
                }
                "RBrack" => {
                    if depth == 0 {
                        return Err(ToyError::new(
                            ToyErrorType::UnclosedDelimiter,
                            Some(raw_text.clone()),
                        ));
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
                return Err(ToyError::new(
                    ToyErrorType::UnknownSymbol(t.clone()),
                    Some(raw_text.clone()),
                ));
            }

            let val = *maybe_val.unwrap();
            if val <= best_val {
                best_val = val;
                best_idx = i;
                best_tok = t.clone();
            }
        }

        if depth != 0 {
            return Err(ToyError::new(
                ToyErrorType::UnclosedDelimiter,
                Some(raw_text.clone()),
            ));
        }

        return Ok((best_idx, best_val, best_tok));
    }

    fn parse_func_call(&self, toks: &Vec<Token>) -> Result<(Ast, TypeTok), ToyError> {
        let raw_text = toks
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        if toks[0].tok_type() != "VarRef" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFuncCall,
                Some(raw_text.clone()),
            ));
        }
        if toks[1].tok_type() != "LParen" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFuncCall,
                Some(raw_text.clone()),
            ));
        }
        if toks.last().unwrap().tok_type() != "RParen" {
            return Err(ToyError::new(
                ToyErrorType::MalformedFuncCall,
                Some(raw_text.clone()),
            ));
        }
        let name = match toks[0].clone() {
            Token::VarRef(n) => *n,
            _ => unreachable!(),
        };
        let param_toks = &toks[2..toks.len() - 1];
        let mut unprocessed_params: Vec<Vec<Token>> = Vec::new();

        if !param_toks.is_empty() {
            let mut current_param: Vec<Token> = Vec::new();
            let mut paren_depth = 0;
            let mut brace_depth = 0;
            let mut brack_depth = 0;

            for t in param_toks {
                match t {
                    Token::LParen => paren_depth += 1,
                    Token::RParen => paren_depth -= 1,
                    Token::LBrace => brace_depth += 1,
                    Token::RBrace => brace_depth -= 1,
                    Token::LBrack => brack_depth += 1,
                    Token::RBrack => brack_depth -= 1,
                    Token::Comma if paren_depth == 0 && brace_depth == 0 && brack_depth == 0 => {
                        unprocessed_params.push(current_param);
                        current_param = Vec::new();
                        continue;
                    }
                    _ => {}
                }
                current_param.push(t.clone());
            }
            unprocessed_params.push(current_param);
        }

        let mut processed_params: Vec<(Ast, TypeTok)> = Vec::new();
        for p in unprocessed_params {
            processed_params.push(self.parse_expr(&p)?);
        }

        let mut resolved_name = name.clone();
        let mut types_opt = self.func_param_type_map.get(&name);

        if types_opt.is_none() && !processed_params.is_empty() {
            if let (_, TypeTok::Struct(fields)) = &processed_params[0] {
                if let Some(struct_name) = self.struct_type_to_name.get(fields) {
                    let method_name = format!("{}:::{}", struct_name, name);
                    if self.func_param_type_map.contains_key(&method_name) {
                        resolved_name = method_name;
                        types_opt = self.func_param_type_map.get(&resolved_name);
                    }
                }
            }
        }
        let builtins = vec!["print", "println", "len", "str", "bool", "int", "float", "input"];
        if !builtins.contains(&name.as_str()) {
            for (_, t) in processed_params.clone() {
                resolved_name = format!("{}_{}", resolved_name, t.type_str().to_lowercase());
            }
        }
        if types_opt.is_none() {
            types_opt = self.func_param_type_map.get(&resolved_name);
        }

        if types_opt.is_some() && !self.func_return_type_map.contains_key(&resolved_name) {
             if self.func_return_type_map.contains_key(&name) {
                 resolved_name = name.clone();
             }
        }

        if types_opt.is_none() {
            return Err(ToyError::new(
                ToyErrorType::UndefinedFunction,
                Some(raw_text.clone()),
            ));
        }

        let types = types_opt.unwrap();
        for (i, (_, type_tok)) in processed_params.iter().enumerate() {
            if type_tok != &types[i] && types[i] != TypeTok::Any {
                return Err(ToyError::new(
                    ToyErrorType::TypeMismatch,
                    Some(raw_text.clone()),
                ));
            }
        }
        let vals: Vec<Ast> = processed_params
            .iter()
            .filter_map(|ast| {
                let (a, _) = ast;
                Some(a.clone())
            })
            .collect();
        return Ok((
            Ast::FuncCall(Box::new(resolved_name.clone()), vals, raw_text),
            self.func_return_type_map
                .get(&resolved_name.clone())
                .unwrap()
                .clone(),
        ));
    }

    pub fn eat(&mut self) {
        self.bp += 1;
    }

    fn parse_var_dec(
        &mut self,
        name: &Token,
        val: &Vec<Token>,
        var_type: Option<TypeTok>,
    ) -> Result<Ast, ToyError> {
        let name_str = *name.get_var_name().unwrap();
        let (val_ast, val_type) = self.parse_expr(val)?;
        let ret_var_type: TypeTok;

        if var_type.is_some() {
            ret_var_type = var_type.unwrap();
        } else {
            ret_var_type = val_type;
        }
        let raw_text = val
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        let node = Ast::VarDec(
            Box::new(name_str.clone()),
            ret_var_type.clone(),
            Box::new(val_ast),
            raw_text,
        );
        self.insert_var_type(name_str.clone(), ret_var_type.clone());
        return Ok(node);
    }

    fn parse_var_ref(&self, name: &Token) -> Result<Ast, ToyError> {
        let name_s: String;
        match name {
            Token::VarRef(box_str) => name_s = *box_str.clone(),
            _ => unreachable!(),
        }
        return Ok(Ast::VarRef(Box::new(name_s.clone()), name_s));
    }

    fn parse_if_stmt(&mut self, stmt: TBox, should_eat: bool) -> Result<Ast, ToyError> {
        let (cond, body, alt, raw_text) = match stmt {
            TBox::IfStmt(c, b, a, rt) => (c, b, a, rt),
            _ => unreachable!(),
        };

        let (b_cond, b_type) = self.parse_expr(&cond)?;
        if b_type != TypeTok::Bool {
            return Err(ToyError::new(
                ToyErrorType::ExpressionNotBoolean,
                Some(raw_text.clone()),
            ));
        }

        self.push_scope();
        let mut stmt_vec: Vec<Ast> = Vec::new();
        for stmt in body {
            debug!(targets: ["parser_verbose"], stmt);
            stmt_vec.push(self.parse_stmt(stmt, false)?);
        }
        self.pop_scope()?;

        let mut else_val: Option<Vec<Ast>> = None;
        if alt.is_some() {
            self.push_scope();
            let mut else_vec: Vec<Ast> = Vec::new();
            for stmt in alt.unwrap() {
                else_vec.push(self.parse_stmt(stmt, false)?);
            }
            self.pop_scope()?;
            else_val = Some(else_vec);
        }
        let if_stmt = Ast::IfStmt(Box::new(b_cond), stmt_vec, else_val, raw_text);

        if should_eat {
            self.eat();
        }

        return Ok(if_stmt);
    }

    fn parse_extern_func_dec(&mut self, stmt: TBox, should_eat: bool) -> Result<Ast, ToyError> {
        let (name_tok, params, return_type, raw_text) = match stmt {
            TBox::ExternFuncDec(n, p, r, rt) => (n, p, r, rt),
            _ => unreachable!(),
        };
        let name = match name_tok {
            Token::VarName(n) => *n,
            _ => unreachable!(),
        };

        let mut ast_params: Vec<Ast> = Vec::new();
        let mut param_types: Vec<TypeTok> = Vec::new();

        for param in params {
            let (param_name, param_type, param_raw_text) = match param {
                TBox::FuncParam(name, type_tok, rt) => {
                    let n = match name {
                        Token::VarRef(var) => *var,
                        _ => unreachable!(),
                    };
                    (n, type_tok, rt)
                }
                _ => unreachable!(),
            };

            ast_params.push(Ast::FuncParam(
                Box::new(param_name.clone()),
                param_type.clone(),
                param_raw_text,
            ));
            param_types.push(param_type.clone());
        }

        self.extern_funcs.insert(name.clone());
        self.func_param_type_map.insert(name.clone(), param_types);
        self.func_return_type_map
            .insert(name.clone(), return_type.clone());

        if should_eat {
            self.eat();
        }

        return Ok(Ast::ExternFuncDec(
            Box::new(name),
            ast_params,
            return_type,
            raw_text,
        ));
    }

    fn parse_func_dec(&mut self, stmt: TBox, should_eat: bool) -> Result<Ast, ToyError> {
        let (name_tok, params, return_type, box_boxy, raw_text) = match stmt {
            TBox::FuncDec(n, p, r, b, rt) => (n, p, r, b, rt),
            _ => unreachable!(),
        };
        let name = match name_tok {
            Token::VarName(n) => *n,
            _ => unreachable!(),
        };

        let mut ast_params: Vec<Ast> = Vec::new();
        let mut param_types: Vec<TypeTok> = Vec::new();

        for param in params {
            let (param_name, param_type, param_raw_text) = match param {
                TBox::FuncParam(name, type_tok, rt) => {
                    let n = match name {
                        Token::VarRef(var) => *var,
                        _ => unreachable!(),
                    };
                    (n, type_tok, rt)
                }
                _ => unreachable!(),
            };

            ast_params.push(Ast::FuncParam(
                Box::new(param_name.clone()),
                param_type.clone(),
                param_raw_text,
            ));
            param_types.push(param_type.clone());
        }

        self.func_param_type_map.insert(name.clone(), param_types);
        self.func_return_type_map
            .insert(name.clone(), return_type.clone());

        self.push_scope();
        for param_ast in &ast_params {
            if let Ast::FuncParam(param_name, param_type, _) = param_ast {
                self.insert_var_type((**param_name).clone(), param_type.clone());
            }
        }

        let mut body: Vec<Ast> = Vec::new();
        for stmt in box_boxy {
            body.push(self.parse_stmt(stmt, false)?)
        }

        self.pop_scope()?;

        if should_eat {
            self.eat();
        }

        return Ok(Ast::FuncDec(
            Box::new(name),
            ast_params,
            return_type,
            body,
            raw_text,
        ));
    }

    fn parse_stmt(&mut self, val: TBox, should_eat: bool) -> Result<Ast, ToyError> {
        debug!(targets: ["parser_verbose"], val);

        let node = match val {
            TBox::Expr(i, _) => {
                let (node, _) = self.parse_expr(&i)?;
                node
            }
            TBox::VarDec(name, var_type, v_val, _) => {
                self.parse_var_dec(&name, &v_val, var_type.clone())?
            }
            TBox::Assign(lhs, rhs, raw_text) => {
                let (lhs_node, _) = self.parse_expr(&lhs)?;
                let (rhs_node, _) = self.parse_expr(&rhs)?;
                Ast::Assignment(Box::new(lhs_node), Box::new(rhs_node), raw_text)
            }
            TBox::IfStmt(_, _, _, _) => {
                return self.parse_if_stmt(val, should_eat);
            }
            TBox::FuncDec(_, _, _, _, _) => return self.parse_func_dec(val, should_eat),
            TBox::ExternFuncDec(_, _, _, _) => return self.parse_extern_func_dec(val, should_eat),
            TBox::Return(val, raw_text) => {
                let expr = match *val {
                    TBox::Expr(ref v, _) => v,
                    _ => return Err(ToyError::new(ToyErrorType::ExpectedExpression, None)),
                };

                let (res, _) = self.parse_expr(expr)?;
                return Ok(Ast::Return(Box::new(res), raw_text));
            }

            TBox::While(expr, body, raw_text) => {
                let parsed_expr = self.parse_bool_expr(&expr);
                let mut parsed_body: Vec<Ast> = Vec::new();
                for stmt in body {
                    parsed_body.push(self.parse_stmt(stmt, false)?)
                }
                if should_eat {
                    self.eat();
                }
                return Ok(Ast::WhileStmt(
                    Box::new(parsed_expr?),
                    parsed_body,
                    raw_text,
                ));
            }
            TBox::Continue => Ast::Continue,
            TBox::Break => Ast::Break,
            TBox::StructInterface(name, types, raw_text) => {
                let boxed: BTreeMap<String, Box<TypeTok>> = (*types)
                    .clone()
                    .into_iter()
                    .map(|(k, v)| (k, Box::new(v)))
                    .collect();
                self.insert_var_type((*name).clone(), TypeTok::Struct(boxed.clone()));
                // Store mapping from struct type signature to struct name for method resolution
                self.struct_type_to_name.insert(boxed, (*name).clone());

                Ast::StructInterface(name, types, raw_text)
            }
            TBox::ImportStmt(name, raw_text) => {
                self.imports.insert(name.clone(), name.clone());
                if name.contains('.') {
                    let parts: Vec<&str> = name.split('.').collect();
                    if let Some(alias) = parts.last() {
                        self.imports.insert(alias.to_string(), name.clone());
                    }
                }
                self.load_module(&name);
                Ast::ImportStmt(name, raw_text)
            }

            _ => todo!("Unimplemented statement {}", val),
        };

        if should_eat {
            self.eat();
        }

        return Ok(node);
    }

    pub fn generate(&mut self, boxes: Vec<TBox>) -> Result<Vec<Ast>, ToyError> {
        self.boxes = boxes.clone();
        self.bp = 0_usize;
        debug!(targets: ["parser_verbose"], boxes);
        while self.bp < self.boxes.len() {
            let val = self.boxes[self.bp].clone();
            debug!(targets: ["parser_verbose"], val);
            let stmt = self.parse_stmt(val, true)?;
            self.nodes.push(stmt)
        }
        return Ok(self.nodes.clone());
    }
}
#[cfg(test)]
mod test;
