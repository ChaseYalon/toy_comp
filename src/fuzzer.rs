use crate::errors::Span;
use crate::*;
use rand::RngExt;
use rand::{
    SeedableRng,
    distr::{Alphabetic, SampleString},
    rngs::StdRng,
};
use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;
fn var_referenced_in(name: &str, node: &Ast) -> bool {
    match node {
        Ast::VarRef(n, _) => **n == name,
        Ast::VarDec(_, _, expr, _) => var_referenced_in(name, expr),
        Ast::FuncDec(_, _, _, body, _) => body.iter().any(|s| var_referenced_in(name, s)),
        Ast::IfStmt(cond, body, else_body, _) => {
            var_referenced_in(name, cond)
                || body.iter().any(|s| var_referenced_in(name, s))
                || else_body
                    .as_ref()
                    .map(|b| b.iter().any(|s| var_referenced_in(name, s)))
                    .unwrap_or(false)
        }
        Ast::WhileStmt(cond, body, _) => {
            var_referenced_in(name, cond) || body.iter().any(|s| var_referenced_in(name, s))
        }
        Ast::Return(expr, _) => var_referenced_in(name, expr),
        Ast::Assignment(lhs, rhs, _) => {
            var_referenced_in(name, lhs) || var_referenced_in(name, rhs)
        }
        Ast::FuncCall(_, args, _) => args.iter().any(|a| var_referenced_in(name, a)),
        Ast::InfixExpr(l, r, _, _) => {
            var_referenced_in(name, l) || var_referenced_in(name, r)
        }
        Ast::Not(e, _) | Ast::EmptyExpr(e, _) => var_referenced_in(name, e),
        Ast::ArrLit(_, elems, _) => elems.iter().any(|e| var_referenced_in(name, e)),
        Ast::StructLit(_, fields, _) => {
            fields.values().any(|(e, _)| var_referenced_in(name, e))
        }
        Ast::MemberAccess(e, _, _) => var_referenced_in(name, e),
        Ast::IndexAccess(e, idx, _) => {
            var_referenced_in(name, e) || var_referenced_in(name, idx)
        }
        _ => false,
    }
}

struct Scope {
    /// type -> Vec<VarNames>. The names are in no particular order, and one should be selected at random
    /// Also included here are any function parameters that are in scope
    vars: HashMap<TypeTok, Vec<String>>,
}

pub struct TestRunner {
    ///stack of scopes, all starting with the root scope
    scopes: Vec<Scope>,
    ///the rng used for every bit of randomness in the fuzzer, if the test needs to be replayed, just save this rng
    rng: StdRng,
    rng_seed: u64,
    max_stmt_depth: usize,
    max_expr_depth: usize,
    prgm_length: usize,
    program: Vec<Ast>,
    type_tok_range: RangeInclusive<usize>,
    ///struct interfaces
    interfaces: Vec<Ast>,
    ///(ret type, param types, function name)
    functions: Vec<(TypeTok, Vec<TypeTok>, String)>,
    ///only used for delta debugging, Var Name -> Var Type
    var_names_to_types: HashMap<String, TypeTok>,
}

impl TestRunner {
    pub fn new() -> TestRunner {
        Self::new_with_seed(100u64)
    }

    pub fn new_with_seed(seed: u64) -> TestRunner {
        let root = Scope {
            vars: HashMap::new(),
        };
        return TestRunner {
            scopes: vec![root],
            rng: StdRng::seed_from_u64(seed),
            max_stmt_depth: 6,
            max_expr_depth: 2,
            program: vec![],
            rng_seed: seed,
            type_tok_range: 0..=3,
            prgm_length: 20,
            interfaces: vec![],
            functions: vec![],
            var_names_to_types: HashMap::new(),
        };
    }
    fn _random_type(&mut self) -> TypeTok {
        return match self.rng.random_range(self.type_tok_range.clone()) {
            0 => TypeTok::Int,
            1 => TypeTok::Bool,
            2 => TypeTok::Float,
            3 => TypeTok::Str,
            _ => unreachable!(),
        };
    }
    fn _random_field_type(&mut self) -> TypeTok {
        match self.rng.random_range(0..=7) {
            0 => TypeTok::Int,
            1 => TypeTok::Bool,
            2 => TypeTok::Float,
            3 => TypeTok::Str,
            4 => TypeTok::IntArr(1),
            5 => TypeTok::BoolArr(1),
            6 => TypeTok::FloatArr(1),
            7 => TypeTok::StrArr(1),
            _ => unreachable!(),
        }
    }
    fn _random_param_type(&mut self) -> TypeTok {
        match self.rng.random_range(0..=8) {
            0 => TypeTok::Int,
            1 => TypeTok::Bool,
            2 => TypeTok::Float,
            3 => TypeTok::Str,
            4 => TypeTok::IntArr(1),
            5 => TypeTok::BoolArr(1),
            6 => TypeTok::FloatArr(1),
            7 => TypeTok::StrArr(1),
            8 => TypeTok::StrArr(2),
            _ => unreachable!(),
        }
    }
    fn _random_return_type(&mut self) -> TypeTok {
        match self.rng.random_range(0..=7) {
            0 => TypeTok::Int,
            1 => TypeTok::Bool,
            2 => TypeTok::Float,
            3 => TypeTok::Str,
            4 => TypeTok::IntArr(1),
            5 => TypeTok::BoolArr(1),
            6 => TypeTok::FloatArr(1),
            7 => TypeTok::StrArr(1),
            _ => unreachable!(),
        }
    }
    fn _rand_int_infix_op(&mut self) -> InfixOp {
        return match self.rng.random_range(0..=4) {
            0 => InfixOp::Plus,
            1 => InfixOp::Minus,
            2 => InfixOp::Multiply,
            3 => InfixOp::Divide,
            4 => InfixOp::Modulo,
            _ => unreachable!(),
        };
    }
    fn _rand_comp_infix_op(&mut self) -> InfixOp {
        return match self.rng.random_range(0..=5) {
            0 => InfixOp::LessThan,
            1 => InfixOp::GreaterThan,
            2 => InfixOp::LessThanEqt,
            3 => InfixOp::GreaterThanEqt,
            4 => InfixOp::Equals,
            5 => InfixOp::NotEquals,
            _ => unreachable!(),
        };
    }
    fn _rand_bool_infix_op(&mut self) -> InfixOp {
        return match self.rng.random_range(0..=1) {
            0 => InfixOp::And,
            1 => InfixOp::Or,
            _ => unreachable!(),
        };
    }
    fn gen_str_expr(&mut self, depth: usize) -> Ast {
        if depth > self.max_expr_depth {
            return Ast::StringLit(
                Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                Span::null_span(),
            );
        }
        let val = match self.rng.random_range(0..=6) {
            0 => Ast::StringLit(
                Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                Span::null_span(),
            ),
            1 => Ast::InfixExpr(
                Box::new(self.gen_str_expr(depth + 1)),
                Box::new(self.gen_str_expr(depth + 1)),
                InfixOp::Plus,
                Span::null_span(),
            ),
            2 => Ast::EmptyExpr(Box::new(self.gen_str_expr(depth + 1)), Span::null_span()),
            3 => {
                let candidate_variables: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|scope| scope.vars.get(&TypeTok::Str).into_iter().flatten())
                    .cloned()
                    .collect();
                if candidate_variables.len() == 0 {
                    return Ast::StringLit(
                        Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                        Span::null_span(),
                    );
                }
                let v = candidate_variables[self.rng.random_range(0..candidate_variables.len())]
                    .clone();

                Ast::VarRef(Box::new(v), Span::null_span())
            }
            4 => {
                //nested structs are currently not handled, that should be done in the future
                let candidate_variables: Vec<(String, String)> =
                    self.scopes
                        .iter()
                        .flat_map(|scope| scope.vars.iter())
                        .filter_map(|(ty, names)| {
                            if let TypeTok::Struct(struct_ty) = ty {
                                Some((struct_ty, names))
                            } else {
                                None
                            }
                        })
                        .flat_map(|(struct_ty, names)| {
                            names.iter().flat_map(move |var_name| {
                                struct_ty.iter().filter(|(_, t)| ***t == TypeTok::Str).map(
                                    move |(field_name, _)| (var_name.clone(), field_name.clone()),
                                )
                            })
                        })
                        .collect();
                if candidate_variables.len() == 0 {
                    return Ast::StringLit(
                        Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                        Span::null_span(),
                    );
                }
                let (v, m) = candidate_variables
                    [self.rng.random_range(0..candidate_variables.len())]
                .clone();
                Ast::MemberAccess(
                    Box::new(Ast::VarRef(Box::new(v), Span::null_span())),
                    m,
                    Span::null_span(),
                )
            }
            5 => {
                let screw_rust = self.functions.clone();
                let candidate_variables: Vec<&(TypeTok, Vec<TypeTok>, String)> = screw_rust
                    .iter()
                    .filter(|(r, _, _)| *r == TypeTok::Str)
                    .collect();
                if candidate_variables.len() == 0 {
                    return Ast::StringLit(
                        Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                        Span::null_span(),
                    );
                }

                let (_, params, name) =
                    candidate_variables[self.rng.random_range(0..candidate_variables.len())];
                let params = params.clone();
                let name = name.clone();
                let mut ast_params: Vec<Ast> = vec![];
                for p in &params {
                    let v = self.gen_arg_for_type(p, depth + 1);
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name), ast_params, Span::null_span())
            }
            6 => match self.gen_rand_read(&TypeTok::Str) {
                Some(expr) => expr,
                None => Ast::StringLit(
                    Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                    Span::null_span(),
                ),
            },

            _ => unreachable!(),
        };
        return val;
    }
    fn gen_bool_expr(&mut self, depth: usize) -> Ast {
        if depth > self.max_expr_depth {
            return Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span());
        }
        let val = match self.rng.random_range(0..=8) {
            //for right now it does not do function calls
            0 => Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span()),
            1 => Ast::InfixExpr(
                Box::new(self.gen_bool_expr(depth + 1)),
                Box::new(self.gen_bool_expr(depth + 1)),
                self._rand_bool_infix_op(),
                Span::null_span(),
            ),
            2 => Ast::InfixExpr(
                Box::new(self.gen_num_expr(depth + 1)),
                Box::new(self.gen_num_expr(depth + 1)),
                self._rand_comp_infix_op(),
                Span::null_span(),
            ),
            3 => Ast::Not(Box::new(self.gen_bool_expr(depth + 1)), Span::null_span()),
            4 => Ast::EmptyExpr(Box::new(self.gen_bool_expr(depth + 1)), Span::null_span()),
            5 => {
                let candidate_variables: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|scope| scope.vars.get(&TypeTok::Bool).into_iter().flatten())
                    .cloned()
                    .collect();
                if candidate_variables.len() == 0 {
                    return self.gen_bool_expr(depth);
                }
                let v = candidate_variables[self.rng.random_range(0..candidate_variables.len())]
                    .clone();

                Ast::VarRef(Box::new(v), Span::null_span())
            }
            6 => {
                //nested structs are currently not handled, that should be done in the future
                let candidate_variables: Vec<(String, String)> =
                    self.scopes
                        .iter()
                        .flat_map(|scope| scope.vars.iter())
                        .filter_map(|(ty, names)| {
                            if let TypeTok::Struct(struct_ty) = ty {
                                Some((struct_ty, names))
                            } else {
                                None
                            }
                        })
                        .flat_map(|(struct_ty, names)| {
                            names.iter().flat_map(move |var_name| {
                                struct_ty.iter().filter(|(_, t)| ***t == TypeTok::Bool).map(
                                    move |(field_name, _)| (var_name.clone(), field_name.clone()),
                                )
                            })
                        })
                        .collect();
                if candidate_variables.len() == 0 {
                    return self.gen_bool_expr(depth);
                }
                let (v, m) = candidate_variables
                    [self.rng.random_range(0..candidate_variables.len())]
                .clone();
                Ast::MemberAccess(
                    Box::new(Ast::VarRef(Box::new(v), Span::null_span())),
                    m,
                    Span::null_span(),
                )
            }
            7 => {
                let screw_rust = self.functions.clone();
                let candidate_variables: Vec<&(TypeTok, Vec<TypeTok>, String)> = screw_rust
                    .iter()
                    .filter(|(r, _, _)| *r == TypeTok::Bool)
                    .collect();
                if candidate_variables.len() == 0 {
                    return Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span());
                }

                let (_, params, name) =
                    candidate_variables[self.rng.random_range(0..candidate_variables.len())];
                let params = params.clone();
                let name = name.clone();
                let mut ast_params: Vec<Ast> = vec![];
                for p in &params {
                    let v = self.gen_arg_for_type(p, depth + 1);
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name), ast_params, Span::null_span())
            }
            8 => match self.gen_rand_read(&TypeTok::Bool) {
                Some(expr) => expr,
                None => Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span()),
            },
            _ => unreachable!(),
        };
        return val;
    }
    fn gen_if_stmt(&mut self, stmt_depth: usize) -> Vec<Ast> {
        if stmt_depth > self.max_stmt_depth {
            return self.gen_stmt(stmt_depth);
        }
        let block_len = self.rng.random_range(1..=3); //this is small but it prevents a huge exponential branching
        let expr = self.gen_bool_expr(0);
        let mut stmts: Vec<Ast> = vec![];
        self.scopes.push(Scope {
            vars: HashMap::new(),
        });
        for _ in 0..block_len {
            stmts.extend(self.gen_stmt(stmt_depth + 1));
        }
        self.scopes.pop();

        let else_stmts = if self.rng.random_bool(0.5) {
            self.scopes.push(Scope {
                vars: HashMap::new(),
            });
            let mut else_stmts = vec![];
            for _ in 0..block_len {
                else_stmts.extend(self.gen_stmt(stmt_depth + 1));
            }
            self.scopes.pop();
            Some(else_stmts)
        } else {
            None
        };

        return vec![Ast::IfStmt(Box::new(expr), stmts, else_stmts, Span::null_span())];
    }
    fn gen_struct_expr(&mut self, depth: usize) -> (Ast, TypeTok) {
        //make a struct interface - right now each struct has its own interface
        let field_count = self.rng.random_range(1..=5);

        let mut field_types: BTreeMap<String, TypeTok> = BTreeMap::new();

        for _ in 0..field_count {
            let field_name = Alphabetic.sample_string(&mut self.rng, 10);
            field_types.insert(field_name, self._random_field_type());
        }

        let interface_name = Alphabetic.sample_string(&mut self.rng, 10);

        let mut ty: BTreeMap<String, Box<TypeTok>> = BTreeMap::new();

        for (n, v) in &field_types {
            ty.insert(n.clone(), Box::new(v.clone()));
        }
        self.interfaces.push(Ast::StructInterface(
            Box::new(interface_name.clone()),
            Box::new(field_types.clone()),
            Span::null_span(),
        ));

        let mut fields: BTreeMap<String, (Ast, TypeTok)> = BTreeMap::new();

        let field_entries: Vec<(String, TypeTok)> = field_types
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (n, t) in field_entries {
            let v = self.gen_arg_for_type(&t, depth + 1);
            fields.insert(n, (v, t));
        }

        let struct_ty = TypeTok::Struct(ty);

        return (
            Ast::StructLit(
                Box::new(interface_name),
                Box::new(fields),
                Span::null_span(),
            ),
            struct_ty,
        );
    }
    fn gen_while_stmt(&mut self, stmt_depth: usize) -> Vec<Ast> {
        if stmt_depth > self.max_stmt_depth {
            return self.gen_stmt(stmt_depth);
        }
        let block_len = self.rng.random_range(0..=3);
        let expr = self.gen_bool_expr(0);
        self.scopes.push(Scope {
            vars: HashMap::new(),
        });
        let mut stmts: Vec<Ast> = vec![];
        for _ in 0..block_len {
            stmts.extend(self.gen_stmt(stmt_depth + 1));
        }
        self.scopes.pop();
        //counter stuff
        let name = Alphabetic.sample_string(&mut self.rng, 10);
        let counter = Ast::VarDec(
            Box::new(name.clone()),
            TypeTok::Int,
            Box::new(Ast::IntLit(0, Span::null_span())),
            Span::null_span(),
        );
        let if_stmt = Ast::IfStmt(
            Box::new(Ast::InfixExpr(
                Box::new(Ast::VarRef(Box::new(name.clone()), Span::null_span())),
                Box::new(Ast::IntLit(100, Span::null_span())),
                InfixOp::GreaterThanEqt,
                Span::null_span(),
            )),
            vec![Ast::Break(Span::null_span())],
            Some(vec![Ast::Assignment(
                Box::new(Ast::VarRef(Box::new(name.clone()), Span::null_span())),
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new(name), Span::null_span())),
                    Box::new(Ast::IntLit(1, Span::null_span())),
                    InfixOp::Plus,
                    Span::null_span(),
                )),
                Span::null_span(),
            )]),
            Span::null_span(),
        );
        stmts.push(if_stmt);
        return vec![counter, Ast::WhileStmt(Box::new(expr), stmts, Span::null_span())];
    }
    fn gen_function(&mut self) -> Ast {
        let param_count = self.rng.random_range(0..=4);
        // 6g: occasionally make the return type StrArr(2) for the tri-level encapsulation case
        let ret_type = if self.rng.random_range(0..=9) == 0 {
            TypeTok::StrArr(2)
        } else {
            self._random_return_type()
        };
        let mut param_names: Vec<String> = vec![];
        for _ in 0..param_count {
            param_names.push(Alphabetic.sample_string(&mut self.rng, 10));
        }
        let mut param_types: Vec<TypeTok> = vec![];
        for _ in 0..param_count {
            param_types.push(self._random_param_type());
        }
        let mut params: Vec<Ast> = vec![];
        for i in 0..param_count {
            params.push(Ast::FuncParam(
                Box::new(param_names[i].clone()),
                param_types[i].clone(),
                Span::null_span(),
            ));
        }
        let mut body: Vec<Ast> = vec![];
        self.scopes.push(Scope {
            vars: HashMap::new(),
        });
        for i in 0..param_count {
            self.scopes
                .last_mut()
                .unwrap()
                .vars
                .entry(param_types[i].clone())
                .or_insert_with(Vec::new)
                .push(param_names[i].clone());
        }
        for _ in 0..self.rng.random_range(0..=10) {
            body.extend(self.gen_stmt(1));
        }
        self.scopes.pop();
        if ret_type != TypeTok::Void {
            let ret = self.gen_arg_for_type(&ret_type.clone(), 0);
            body.push(Ast::Return(Box::new(ret), Span::null_span()));
        }
        let function_name = Alphabetic.sample_string(&mut self.rng, 10);
        self.functions
            .push((ret_type.clone(), param_types.clone(), function_name.clone()));
        return Ast::FuncDec(
            Box::new(function_name),
            params,
            ret_type,
            body,
            Span::null_span(),
        );
    }
    fn gen_stmt(&mut self, stmt_depth: usize) -> Vec<Ast> {
        if stmt_depth == 0 {
            let f = self.gen_function();
            return vec![f];
        }
        if stmt_depth > self.max_stmt_depth {
            return vec![self.gen_var_dec()]; //this is a bodge
        }
        return match self.rng.random_range(0..=4) {
            0 => vec![self.gen_var_dec()],
            1 => self.gen_if_stmt(stmt_depth),
            2 => self.gen_while_stmt(stmt_depth),
            3 => vec![self
                .gen_arr_elem_write()
                .unwrap_or_else(|| self.gen_var_dec())],
            4 => vec![self
                .gen_struct_field_write()
                .unwrap_or_else(|| self.gen_var_dec())],
            _ => unreachable!(),
        };
    }
    fn gen_int_expr(&mut self, depth: usize) -> Ast {
        if depth > self.max_expr_depth {
            return Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span());
        }
        let val = match self.rng.random_range(0..=6) {
            //for right now it does not do function calls
            0 => Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span()),
            1 => Ast::InfixExpr(
                Box::new(self.gen_int_expr(depth + 1)),
                Box::new(self.gen_int_expr(depth + 1)),
                self._rand_int_infix_op(),
                Span::null_span(),
            ),
            2 => Ast::EmptyExpr(Box::new(self.gen_int_expr(depth + 1)), Span::null_span()),
            3 => {
                let candidate_variables: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|scope| scope.vars.get(&TypeTok::Int).into_iter().flatten())
                    .cloned()
                    .collect();
                if candidate_variables.len() == 0 {
                    return self.gen_int_expr(depth);
                }
                let v = candidate_variables[self.rng.random_range(0..candidate_variables.len())]
                    .clone();
                if candidate_variables.len() == 0 {
                    return self.gen_int_expr(depth);
                }
                Ast::VarRef(Box::new(v), Span::null_span())
            }
            4 => {
                //nested structs are currently not handled, that should be done in the future
                let candidate_variables: Vec<(String, String)> =
                    self.scopes
                        .iter()
                        .flat_map(|scope| scope.vars.iter())
                        .filter_map(|(ty, names)| {
                            if let TypeTok::Struct(struct_ty) = ty {
                                Some((struct_ty, names))
                            } else {
                                None
                            }
                        })
                        .flat_map(|(struct_ty, names)| {
                            names.iter().flat_map(move |var_name| {
                                struct_ty.iter().filter(|(_, t)| ***t == TypeTok::Int).map(
                                    move |(field_name, _)| (var_name.clone(), field_name.clone()),
                                )
                            })
                        })
                        .collect();
                if candidate_variables.len() == 0 {
                    return self.gen_int_expr(depth);
                }
                let (v, m) = candidate_variables
                    [self.rng.random_range(0..candidate_variables.len())]
                .clone();
                Ast::MemberAccess(
                    Box::new(Ast::VarRef(Box::new(v), Span::null_span())),
                    m,
                    Span::null_span(),
                )
            }
            5 => {
                let screw_rust = self.functions.clone();
                let candidate_variables: Vec<&(TypeTok, Vec<TypeTok>, String)> = screw_rust
                    .iter()
                    .filter(|(r, _, _)| *r == TypeTok::Int)
                    .collect();
                if candidate_variables.len() == 0 {
                    return Ast::IntLit(
                        self.rng.random_range(i64::MIN..i64::MAX),
                        Span::null_span(),
                    );
                }

                let (_, params, name) =
                    candidate_variables[self.rng.random_range(0..candidate_variables.len())];
                let params = params.clone();
                let name = name.clone();
                let mut ast_params: Vec<Ast> = vec![];
                for p in &params {
                    let v = self.gen_arg_for_type(p, depth + 1);
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name), ast_params, Span::null_span())
            }
            6 => match self.gen_rand_read(&TypeTok::Int) {
                Some(expr) => expr,
                None => Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span()),
            },
            _ => unreachable!(),
        };
        return val;
    }
    fn gen_num_expr(&mut self, depth: usize) -> Ast {
        return if self.rng.random_bool(0.5) {
            self.gen_int_expr(depth)
        } else {
            self.gen_float_expr(depth)
        };
    }
    fn gen_float_expr(&mut self, depth: usize) -> Ast {
        if depth > self.max_expr_depth {
            return Ast::FloatLit(
                OrderedFloat(self.rng.random_range(-1_000_000.0..1_000_000.0)),
                Span::null_span(),
            );
        }
        let val = match self.rng.random_range(0..=6) {
            //for right now it does not do function calls
            0 => Ast::FloatLit(
                OrderedFloat(self.rng.random_range(-1_000_000.0..1_000_000.0)),
                Span::null_span(),
            ),
            1 => Ast::InfixExpr(
                Box::new(self.gen_float_expr(depth + 1)),
                Box::new(self.gen_float_expr(depth + 1)),
                self._rand_int_infix_op(),
                Span::null_span(),
            ),
            2 => Ast::EmptyExpr(Box::new(self.gen_float_expr(depth + 1)), Span::null_span()),
            3 => {
                let candidate_variables: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|scope| scope.vars.get(&TypeTok::Float).into_iter().flatten())
                    .cloned()
                    .collect();
                if candidate_variables.len() == 0 {
                    return self.gen_float_expr(depth);
                }
                let v = candidate_variables[self.rng.random_range(0..candidate_variables.len())]
                    .clone();

                Ast::VarRef(Box::new(v), Span::null_span())
            }
            4 => {
                //nested structs are currently not handled, that should be done in the future
                let candidate_variables: Vec<(String, String)> = self
                    .scopes
                    .iter()
                    .flat_map(|scope| scope.vars.iter())
                    .filter_map(|(ty, names)| {
                        if let TypeTok::Struct(struct_ty) = ty {
                            Some((struct_ty, names))
                        } else {
                            None
                        }
                    })
                    .flat_map(|(struct_ty, names)| {
                        names.iter().flat_map(move |var_name| {
                            struct_ty
                                .iter()
                                .filter(|(_, t)| ***t == TypeTok::Float)
                                .map(move |(field_name, _)| (var_name.clone(), field_name.clone()))
                        })
                    })
                    .collect();
                if candidate_variables.len() == 0 {
                    return self.gen_float_expr(depth);
                }
                let (v, m) = candidate_variables
                    [self.rng.random_range(0..candidate_variables.len())]
                .clone();
                Ast::MemberAccess(
                    Box::new(Ast::VarRef(Box::new(v), Span::null_span())),
                    m,
                    Span::null_span(),
                )
            }
            5 => {
                let screw_rust = self.functions.clone();
                let candidate_variables: Vec<&(TypeTok, Vec<TypeTok>, String)> = screw_rust
                    .iter()
                    .filter(|(r, _, _)| *r == TypeTok::Float)
                    .collect();
                if candidate_variables.len() == 0 {
                    return Ast::FloatLit(
                        ordered_float::OrderedFloat(
                            self.rng.random_range(-1_000_000.0..1_000_000.0),
                        ),
                        Span::null_span(),
                    );
                }

                let (_, params, name) =
                    candidate_variables[self.rng.random_range(0..candidate_variables.len())];
                let params = params.clone();
                let name = name.clone();
                let mut ast_params: Vec<Ast> = vec![];
                for p in &params {
                    let v = self.gen_arg_for_type(p, depth + 1);
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name), ast_params, Span::null_span())
            }
            6 => match self.gen_rand_read(&TypeTok::Float) {
                Some(expr) => expr,
                None => Ast::FloatLit(
                    OrderedFloat(self.rng.random_range(-1_000_000.0..1_000_000.0)),
                    Span::null_span(),
                ),
            },
            _ => unreachable!(),
        };
        return val;
    }
    fn gen_expr(&mut self) -> (Ast, TypeTok) {
        let n = self.rng.random_range(0..=8);
        return match n {
            0 => (self.gen_int_expr(0), TypeTok::Int),
            1 => (self.gen_float_expr(0), TypeTok::Float),
            2 => (self.gen_bool_expr(0), TypeTok::Bool),
            3 => self.gen_struct_expr(0),
            4 => {
                let len = self.rng.random_range(1..=25usize);
                (self.gen_arr_expr(TypeTok::Int, len), TypeTok::IntArr(1))
            }
            5 => {
                let len = self.rng.random_range(1..=25usize);
                (self.gen_arr_expr(TypeTok::Bool, len), TypeTok::BoolArr(1))
            }
            6 => {
                let len = self.rng.random_range(1..=25usize);
                (self.gen_arr_expr(TypeTok::Float, len), TypeTok::FloatArr(1))
            }
            7 => {
                let len = self.rng.random_range(1..=25usize);
                (self.gen_arr_expr(TypeTok::Str, len), TypeTok::StrArr(1))
            }
            8 => {
                let len = self.rng.random_range(1..=10usize);
                (self.gen_nested_str_arr_expr(len), TypeTok::StrArr(2))
            }
            _ => todo!("{:?} is unimplemented", n),
        };
    }
    fn gen_var_dec(&mut self) -> Ast {
        let name = Alphabetic.sample_string(&mut self.rng, 10);
        let (v, t) = self.gen_expr();
        self.scopes
            .last_mut()
            .unwrap()
            .vars
            .entry(t.clone())
            .or_insert_with(Vec::new)
            .push(name.clone());
        return Ast::VarDec(Box::new(name), t.clone(), Box::new(v), Span::null_span());
    }
    pub fn generate(&mut self) -> Vec<Ast> {
        self.scopes.push(Scope {
            vars: HashMap::new(),
        });
        for _ in 0..self.prgm_length {
            //for now only var dec
            let v = self.gen_stmt(0);
            self.program.extend(v);
        }
        let funcs = self.program.clone();
        self.interfaces.append(&mut self.program);
        let mut result = vec![Ast::ImportStmt("std.fuzz".to_string(), Span::null_span())];
        result.extend(self.interfaces.iter().cloned());
        for f in &funcs {
            //every top level program is a func
            let (func_name, func_params) = match f {
                Ast::FuncDec(n, p, _, _, _) => (*n.clone(), p),
                _ => continue,
            };
            let mut params: Vec<Ast> = vec![];
            for p in func_params {
                let (_, param_type) = match p {
                    Ast::FuncParam(n, t, _) => (*n.clone(), t),
                    _ => unreachable!(),
                };
                params.push(self.gen_expr_of_type(param_type.clone(), 0));
            }
            result.push(Ast::FuncCall(
                Box::new(func_name),
                params,
                Span::null_span(),
            ));
        }

        return result;
    }
    fn gen_expr_of_type(&mut self, ty: TypeTok, depth: usize) -> Ast {
        match ty {
            TypeTok::Int => self.gen_int_expr(depth),
            TypeTok::Float => self.gen_float_expr(depth),
            TypeTok::Bool => self.gen_bool_expr(depth),
            TypeTok::Str => self.gen_str_expr(depth),
            TypeTok::IntArr(_) => {
                let len = self.rng.random_range(1..=25usize);
                self.gen_arr_expr(TypeTok::Int, len)
            }
            TypeTok::BoolArr(_) => {
                let len = self.rng.random_range(1..=25usize);
                self.gen_arr_expr(TypeTok::Bool, len)
            }
            TypeTok::FloatArr(_) => {
                let len = self.rng.random_range(1..=25usize);
                self.gen_arr_expr(TypeTok::Float, len)
            }
            TypeTok::StrArr(1) => {
                let len = self.rng.random_range(1..=25usize);
                self.gen_arr_expr(TypeTok::Str, len)
            }
            TypeTok::StrArr(2) => {
                let len = self.rng.random_range(1..=10usize);
                self.gen_nested_str_arr_expr(len)
            }
            TypeTok::StrArr(_) => {
                let len = self.rng.random_range(1..=25usize);
                self.gen_arr_expr(TypeTok::Str, len)
            }
            _ => todo!("unsupported replacement type {:?}", ty),
        }
    }
    fn typeof_node(&self, node: &Ast) -> Option<TypeTok> {
        return match node {
            Ast::IntLit(_, _) => Some(TypeTok::Int),
            Ast::FloatLit(_, _) => Some(TypeTok::Float),
            Ast::BoolLit(_, _) => Some(TypeTok::Bool),
            Ast::ArrLit(ty, _, _) => Some(ty.clone()),
            Ast::InfixExpr(l, r, op, _) => {
                if matches!(
                    op,
                    InfixOp::And
                        | InfixOp::Or
                        | InfixOp::NotEquals
                        | InfixOp::Equals
                        | InfixOp::LessThan
                        | InfixOp::LessThanEqt
                        | InfixOp::GreaterThan
                        | InfixOp::GreaterThanEqt
                ) {
                    Some(TypeTok::Bool)
                } else if self.typeof_node(&(**l)) == Some(TypeTok::Float)
                    || self.typeof_node(&(**r)) == Some(TypeTok::Float)
                {
                    Some(TypeTok::Float)
                } else {
                    Some(TypeTok::Int) //infix expr is always bool, float, int
                }
            }
            Ast::EmptyExpr(sub, _) => self.typeof_node(&(**sub)),
            Ast::Not(_, _) => Some(TypeTok::Bool),
            Ast::VarDec(_, t, _, _) => {
                if matches!(t, TypeTok::Int | TypeTok::Float | TypeTok::Bool) {
                    Some(t.clone())
                } else {
                    None
                }
            }
            Ast::VarRef(n, _) => {
                let res = self.var_names_to_types.get(&(**n).clone());
                if res.is_some()
                    && matches!(res.unwrap(), TypeTok::Int | TypeTok::Float | TypeTok::Bool)
                {
                    Some(res.unwrap().clone())
                } else {
                    None
                }
            }
            _ => None, //probably more edge cases not accounted for
        };
    }
    fn rewrite_expr(&mut self, expr: Ast, removed_name: &str, removed_ret: &TypeTok) -> Ast {
        match expr {
            Ast::FuncCall(name, args, span) => {
                if *name == removed_name {
                    return self.gen_expr_of_type(removed_ret.clone(), self.max_expr_depth + 1);
                }

                let param_types: Vec<TypeTok> = self
                    .functions
                    .iter()
                    .find(|(_, _, n)| n == &*name)
                    .map(|(_, params, _)| params.clone())
                    .unwrap_or_default();

                Ast::FuncCall(
                    name,
                    args.into_iter()
                        .enumerate()
                        .map(|(i, a)| {
                            let rewritten = self.rewrite_expr(a, removed_name, removed_ret);
                            if let Some(expected_ty) = param_types.get(i) {
                                match self.typeof_node(&rewritten) {
                                    Some(actual_ty) if actual_ty != *expected_ty => self
                                        .gen_expr_of_type(
                                            expected_ty.clone(),
                                            self.max_expr_depth + 1,
                                        ),
                                    _ => rewritten,
                                }
                            } else {
                                rewritten
                            }
                        })
                        .collect(),
                    span,
                )
            }

            Ast::InfixExpr(lhs, rhs, op, span) => {
                if self.rng.random_range(0..=4) == 0 {
                    if matches!(
                        op,
                        InfixOp::And
                            | InfixOp::Or
                            | InfixOp::Equals
                            | InfixOp::NotEquals
                            | InfixOp::GreaterThan
                            | InfixOp::GreaterThanEqt
                            | InfixOp::LessThan
                            | InfixOp::LessThanEqt
                    ) {
                        Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span())
                    } else if self.typeof_node(&*lhs) == Some(TypeTok::Float)
                        || self.typeof_node(&*rhs) == Some(TypeTok::Float)
                    {
                        Ast::FloatLit(
                            OrderedFloat::from(self.rng.random_range(-1_000_000.0f64..1_000_000.0)),
                            Span::null_span(),
                        )
                    } else if self.typeof_node(&*lhs) == Some(TypeTok::Int)
                        || self.typeof_node(&*rhs) == Some(TypeTok::Int)
                    {
                        Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span())
                    } else {
                        // Type unknown (e.g. FuncCall) — rewrite recursively instead of guessing
                        Ast::InfixExpr(
                            Box::new(self.rewrite_expr(*lhs, removed_name, removed_ret)),
                            Box::new(self.rewrite_expr(*rhs, removed_name, removed_ret)),
                            op,
                            span,
                        )
                    }
                } else {
                    Ast::InfixExpr(
                        Box::new(self.rewrite_expr(*lhs, removed_name, removed_ret)),
                        Box::new(self.rewrite_expr(*rhs, removed_name, removed_ret)),
                        op,
                        span,
                    )
                }
            }

            Ast::EmptyExpr(expr, span) => {
                //higher change to remove EmptyExpr, it is useless
                let ty = self.typeof_node(&*expr);
                if self.rng.random_range(0..=1) == 0 && ty.is_some() {
                    match ty.unwrap() {
                        TypeTok::Bool => Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span()),
                        TypeTok::Float => Ast::FloatLit(
                            OrderedFloat::from(self.rng.random_range(-1_000_000.0f64..1_000_000.0)),
                            Span::null_span(),
                        ),
                        TypeTok::Int => Ast::IntLit(
                            self.rng.random_range(i64::MIN..i64::MAX),
                            Span::null_span(),
                        ),
                        _ => Ast::EmptyExpr(
                            Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                            span,
                        ),
                    }
                } else {
                    Ast::EmptyExpr(
                        Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                        span,
                    )
                }
            }

            Ast::Not(expr, span) => {
                if self.rng.random_range(0..=4) == 0 {
                    Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span())
                } else {
                    Ast::Not(
                        Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                        span,
                    )
                }
            }
            Ast::MemberAccess(expr, field, span) => Ast::MemberAccess(
                Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                field,
                span,
            ),

            Ast::StructLit(name, fields, span) => {
                let new_fields = fields
                    .into_iter()
                    .map(|(k, (expr, ty))| {
                        (k, (self.rewrite_expr(expr, removed_name, removed_ret), ty))
                    })
                    .collect();

                Ast::StructLit(name, Box::new(new_fields), span)
            }

            Ast::ArrLit(ty, elems, span) => {
                let mut new_elems: Vec<Ast> = elems
                    .into_iter()
                    .map(|e| self.rewrite_expr(e, removed_name, removed_ret))
                    .collect();
                // shrink arrays toward length 1
                if new_elems.len() > 1 {
                    let keep = self.rng.random_range(1..=new_elems.len());
                    new_elems.truncate(keep);
                }
                Ast::ArrLit(ty, new_elems, span)
            }

            Ast::StringLit(s, span) => {
                if s.len() > 1 {
                    let new_len = self.rng.random_range(1..=s.len());
                    Ast::StringLit(Box::new(s[..new_len].to_string()), span)
                } else if s.is_empty() {
                    Ast::StringLit(Box::new(Alphabetic.sample_string(&mut self.rng, 1)), span)
                } else {
                    Ast::StringLit(s, span)
                }
            }

            _ => expr,
        }
    }
    fn rewrite_stmt(
        &mut self,
        stmt: Ast,
        removed_name: &str,
        removed_ret: &TypeTok,
    ) -> Option<Ast> {
        match stmt {
            Ast::FuncDec(name, params, ret, body, span) => {
                if *name == removed_name {
                    return None;
                }

                let body = body
                    .into_iter()
                    .filter_map(|s| self.rewrite_stmt(s, removed_name, removed_ret))
                    .collect();

                Some(Ast::FuncDec(name, params, ret, body, span))
            }

            Ast::VarDec(name, ty, expr, span) => {
                self.var_names_to_types.insert((*name).clone(), ty.clone());
                let rewritten = self.rewrite_expr(*expr, removed_name, removed_ret);
                let fixed = match self.typeof_node(&rewritten) {
                    Some(actual_ty) if actual_ty != ty => {
                        self.gen_expr_of_type(ty.clone(), self.max_expr_depth + 1)
                    }
                    _ => rewritten,
                };
                Some(Ast::VarDec(name, ty, Box::new(fixed), span))
            }

            Ast::IfStmt(cond, body, else_body, span) => {
                //give every if stmt a 1/6 chance of being removed
                if self.rng.random_range(0..=5) == 0 || body.len() == 0 {
                    None
                } else {
                    let cond = Box::new(self.rewrite_expr(*cond, removed_name, removed_ret));

                    let body = body
                        .into_iter()
                        .filter_map(|s| self.rewrite_stmt(s, removed_name, removed_ret))
                        .collect();

                    let else_body = else_body.map(|body| {
                        body.into_iter()
                            .filter_map(|s| self.rewrite_stmt(s, removed_name, removed_ret))
                            .collect()
                    });

                    Some(Ast::IfStmt(cond, body, else_body, span))
                }
            }

            Ast::WhileStmt(cond, mut body, span) => {
                //same as if
                if self.rng.random_range(0..=5) == 0 || body.len() == 0 {
                    None
                } else {
                    let cond = Box::new(self.rewrite_expr(*cond, removed_name, removed_ret));

                    // Detect the counter-guard if-stmt (last stmt: if with break
                    // in true branch + assignment in else branch) and preserve it
                    // so the reducer never removes the infinite-loop safeguard.
                    let guard = match body.last() {
                        Some(Ast::IfStmt(_, if_body, Some(else_body), _))
                            if if_body.len() == 1
                                && matches!(if_body[0], Ast::Break(_))
                                && else_body.len() == 1
                                && matches!(else_body[0], Ast::Assignment(..)) =>
                        {
                            Some(body.pop().unwrap())
                        }
                        _ => None,
                    };

                    let mut body: Vec<Ast> = body
                        .into_iter()
                        .filter_map(|s| self.rewrite_stmt(s, removed_name, removed_ret))
                        .collect();

                    if let Some(g) = guard {
                        body.push(g);
                    }

                    Some(Ast::WhileStmt(cond, body, span))
                }
            }

            Ast::Return(expr, span) => Some(Ast::Return(
                Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                span,
            )),

            Ast::FuncCall(name, args, span) => {
                if *name == removed_name {
                    return None;
                }
                let rewritten_args = args
                    .into_iter()
                    .map(|a| self.rewrite_expr(a, removed_name, removed_ret))
                    .collect();
                Some(Ast::FuncCall(name, rewritten_args, span))
            }

            _ => Some(stmt),
        }
    }
    fn gen_arr_elem_write(&mut self) -> Option<Ast> {
        let array_vars: Vec<(TypeTok, String)> = self
            .scopes
            .iter()
            .flat_map(|s| s.vars.iter())
            .filter_map(|(ty, names)| match ty {
                TypeTok::IntArr(1)
                | TypeTok::BoolArr(1)
                | TypeTok::FloatArr(1)
                | TypeTok::StrArr(1)
                | TypeTok::StrArr(2) => Some(
                    names
                        .iter()
                        .map(|n| (ty.clone(), n.clone()))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect();
        if array_vars.is_empty() {
            return None;
        }
        let (arr_type, arr_name) = array_vars[self.rng.random_range(0..array_vars.len())].clone();
        let elem_type = match &arr_type {
            TypeTok::IntArr(1) => TypeTok::Int,
            TypeTok::BoolArr(1) => TypeTok::Bool,
            TypeTok::FloatArr(1) => TypeTok::Float,
            TypeTok::StrArr(1) => TypeTok::Str,
            TypeTok::StrArr(2) => TypeTok::StrArr(1),
            _ => return None,
        };
        let val_expr = self.gen_arg_for_type(&elem_type.clone(), 0);
        let mangled = crate::driver::Driver::mangle_name(
            Some("std::fuzz"),
            "write_arr",
            &[arr_type.clone(), elem_type],
        );
        Some(Ast::FuncCall(
            Box::new(mangled),
            vec![Ast::VarRef(Box::new(arr_name), Span::null_span()), val_expr],
            Span::null_span(),
        ))
    }
    fn gen_struct_field_write(&mut self) -> Option<Ast> {
        let struct_vars: Vec<(TypeTok, String)> = self
            .scopes
            .iter()
            .flat_map(|s| s.vars.iter())
            .filter_map(|(ty, names)| {
                if matches!(ty, TypeTok::Struct(_)) {
                    Some(
                        names
                            .iter()
                            .map(|n| (ty.clone(), n.clone()))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .flatten()
            .collect();
        if struct_vars.is_empty() {
            return None;
        }
        let (struct_type, var_name) =
            struct_vars[self.rng.random_range(0..struct_vars.len())].clone();
        let TypeTok::Struct(fields) = &struct_type else {
            return None;
        };
        let fields_vec: Vec<(String, TypeTok)> = fields
            .iter()
            .map(|(k, v)| (k.clone(), *v.clone()))
            .collect();
        if fields_vec.is_empty() {
            return None;
        }
        let (field_name, field_type) =
            fields_vec[self.rng.random_range(0..fields_vec.len())].clone();
        let val_expr = self.gen_arg_for_type(&field_type, 0);
        Some(Ast::Assignment(
            Box::new(Ast::MemberAccess(
                Box::new(Ast::VarRef(Box::new(var_name), Span::null_span())),
                field_name,
                Span::null_span(),
            )),
            Box::new(val_expr),
            Span::null_span(),
        ))
    }
    fn gen_nested_str_arr_expr(&mut self, outer_len: usize) -> Ast {
        let mut elements: Vec<Ast> = Vec::with_capacity(outer_len);
        for _ in 0..outer_len {
            let inner_len = self.rng.random_range(1..=8usize);
            elements.push(self.gen_arr_expr(TypeTok::Str, inner_len));
        }
        Ast::ArrLit(TypeTok::StrArr(2), elements, Span::null_span())
    }
    /// Generates an expression of any type, including array types, for use as a function argument
    /// or array element. Prefers referencing in-scope variables over generating fresh literals.
    fn gen_arg_for_type(&mut self, ty: &TypeTok, depth: usize) -> Ast {
        match ty {
            TypeTok::Int => self.gen_int_expr(depth),
            TypeTok::Float => self.gen_float_expr(depth),
            TypeTok::Bool => self.gen_bool_expr(depth),
            TypeTok::Str => self.gen_str_expr(depth),
            TypeTok::StrArr(2) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(&TypeTok::StrArr(2)).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                if let Some(expr) = self.gen_rand_read(&TypeTok::StrArr(1)) {
                    let outer_len = self.rng.random_range(1..=5usize);
                    let mut elements = vec![expr];
                    for _ in 1..outer_len {
                        let inner_len = self.rng.random_range(1..=6usize);
                        elements.push(self.gen_literal_arr(TypeTok::Str, inner_len));
                    }
                    return Ast::ArrLit(TypeTok::StrArr(2), elements, Span::null_span());
                }
                let n = self.rng.random_range(1..=5usize);
                let mut elements: Vec<Ast> = Vec::with_capacity(n);
                for _ in 0..n {
                    let inner_len = self.rng.random_range(1..=6usize);
                    elements.push(self.gen_literal_arr(TypeTok::Str, inner_len));
                }
                Ast::ArrLit(TypeTok::StrArr(2), elements, Span::null_span())
            }
            TypeTok::StrArr(1) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(&TypeTok::StrArr(1)).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                if let Some(expr) = self.gen_rand_read(&TypeTok::StrArr(1)) {
                    return expr;
                }
                let n = self.rng.random_range(1..=10usize);
                self.gen_literal_arr(TypeTok::Str, n)
            }
            TypeTok::IntArr(_) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(&TypeTok::IntArr(1)).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                let n = self.rng.random_range(1..=10usize);
                self.gen_literal_arr(TypeTok::Int, n)
            }
            TypeTok::BoolArr(_) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(&TypeTok::BoolArr(1)).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                let n = self.rng.random_range(1..=10usize);
                self.gen_literal_arr(TypeTok::Bool, n)
            }
            TypeTok::FloatArr(_) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(&TypeTok::FloatArr(1)).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                let n = self.rng.random_range(1..=10usize);
                self.gen_literal_arr(TypeTok::Float, n)
            }
            _ => todo!("unsupported arg type {:?}", ty),
        }
    }
    fn gen_literal_arr(&mut self, elem_type: TypeTok, length: usize) -> Ast {
        let mut elements: Vec<Ast> = Vec::with_capacity(length);
        for _ in 0..length {
            let elem = match elem_type {
                TypeTok::Int => {
                    Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span())
                }
                TypeTok::Bool => Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span()),
                TypeTok::Float => Ast::FloatLit(
                    OrderedFloat(self.rng.random_range(-1_000_000.0..1_000_000.0)),
                    Span::null_span(),
                ),
                TypeTok::Str => Ast::StringLit(
                    Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                    Span::null_span(),
                ),
                _ => unreachable!(),
            };
            elements.push(elem);
        }
        let arr_type = match elem_type {
            TypeTok::Int => TypeTok::IntArr(1),
            TypeTok::Bool => TypeTok::BoolArr(1),
            TypeTok::Float => TypeTok::FloatArr(1),
            TypeTok::Str => TypeTok::StrArr(1),
            _ => unreachable!(),
        };
        Ast::ArrLit(arr_type, elements, Span::null_span())
    }
    fn gen_arr_expr(&mut self, elem_type: TypeTok, length: usize) -> Ast {
        let mut elements: Vec<Ast> = Vec::with_capacity(length);
        for _ in 0..length {
            let elem = match elem_type {
                TypeTok::Int => self.gen_int_expr(0),
                TypeTok::Bool => self.gen_bool_expr(0),
                TypeTok::Float => self.gen_float_expr(0),
                TypeTok::Str => self.gen_str_expr(0),
                _ => unreachable!(),
            };
            elements.push(elem);
        }
        let arr_type = match elem_type {
            TypeTok::Int => TypeTok::IntArr(1),
            TypeTok::Bool => TypeTok::BoolArr(1),
            TypeTok::Float => TypeTok::FloatArr(1),
            TypeTok::Str => TypeTok::StrArr(1),
            _ => unreachable!(),
        };
        return Ast::ArrLit(arr_type, elements, Span::null_span());
    }
    fn gen_rand_read(&mut self, elem_type: &TypeTok) -> Option<Ast> {
        let arr_type = match elem_type {
            TypeTok::Int => TypeTok::IntArr(1),
            TypeTok::Bool => TypeTok::BoolArr(1),
            TypeTok::Float => TypeTok::FloatArr(1),
            TypeTok::Str => TypeTok::StrArr(1),
            TypeTok::StrArr(1) => TypeTok::StrArr(2),
            _ => return None,
        };
        let candidates: Vec<String> = self
            .scopes
            .iter()
            .flat_map(|scope| scope.vars.get(&arr_type).into_iter().flatten())
            .cloned()
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let v = candidates[self.rng.random_range(0..candidates.len())].clone();
        let mangled =
            crate::driver::Driver::mangle_name(Some("std::fuzz"), "read_rand", &[arr_type.clone()]);
        return Some(Ast::FuncCall(
            Box::new(mangled),
            vec![Ast::VarRef(Box::new(v), Span::null_span())],
            Span::null_span(),
        ));
    }
    /// Removes a single statement from a randomly chosen function body.
    /// Only removes VarDec nodes whose variable is not referenced anywhere
    /// else in the body, so the result is always a valid, well-scoped program.
    pub fn reduce_body(&mut self, input: Vec<Ast>) -> Vec<Ast> {
        let func_indices: Vec<usize> = input
            .iter()
            .enumerate()
            .filter(|(_, n)| n.node_type() == "FuncDec")
            .map(|(i, _)| i)
            .collect();

        if func_indices.is_empty() {
            return input;
        }

        let func_idx = func_indices[self.rng.random_range(0..func_indices.len())];
        let mut result = input;

        if let Ast::FuncDec(name, params, ret, body, span) = result[func_idx].clone() {
            let removable: Vec<usize> = (0..body.len())
                .filter(|&i| {
                    if matches!(body[i], Ast::Return(..)) {
                        return false;
                    }
                    // Don't remove a VarDec whose variable is still referenced
                    // by other statements — that would leave a dangling VarRef.
                    if let Ast::VarDec(var_name, _, _, _) = &body[i] {
                        !body
                            .iter()
                            .enumerate()
                            .any(|(j, s)| j != i && var_referenced_in(var_name, s))
                    } else {
                        true
                    }
                })
                .collect();

            if removable.is_empty() {
                return result;
            }

            let stmt_idx = removable[self.rng.random_range(0..removable.len())];
            let mut new_body = body;
            new_body.remove(stmt_idx);
            result[func_idx] = Ast::FuncDec(name, params, ret, new_body, span);
        }

        result
    }

    pub fn reduce(&mut self, input: Vec<Ast>) -> Vec<Ast> {
        let funcs: Vec<Ast> = input
            .iter()
            .filter(|n| n.node_type() == "FuncDec")
            .cloned()
            .collect();

        if funcs.is_empty() {
            return input;
        }

        // Repopulate self.functions from the current AST so rewrite_expr can
        // look up expected param types and avoid generating type mismatches.
        self.functions.clear();
        for f in &funcs {
            if let Ast::FuncDec(name, params, ret_type, _, _) = f {
                let param_types: Vec<TypeTok> = params
                    .iter()
                    .filter_map(|p| {
                        if let Ast::FuncParam(_, ty, _) = p {
                            Some(ty.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                self.functions
                    .push((ret_type.clone(), param_types, (**name).clone()));
            }
        }

        let removed = funcs[self.rng.random_range(0..funcs.len())].clone();

        let (removed_name, removed_ret) = match removed {
            Ast::FuncDec(name, _, ret, _, _) => (*name, ret),
            _ => unreachable!(),
        };

        self.functions.retain(|(_, _, n)| *n != removed_name);
        input
            .into_iter()
            .filter_map(|stmt| self.rewrite_stmt(stmt, &removed_name, &removed_ret))
            .collect()
    }
}
