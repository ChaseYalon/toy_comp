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
}

impl TestRunner {
    pub fn new() -> TestRunner {
        let root = Scope {
            vars: HashMap::new(),
        };
        let seed = 100u64; //this is sketchy
        return TestRunner {
            scopes: vec![root],
            rng: StdRng::seed_from_u64(seed),
            max_stmt_depth: 6,
            max_expr_depth: 3,
            program: vec![],
            rng_seed: seed,
            type_tok_range: 0..=3,
            prgm_length: 10,
            interfaces: vec![],
            functions: vec![],
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
        let val = match self.rng.random_range(0..=5) {
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
                let mut ast_params: Vec<Ast> = vec![];
                for p in params {
                    let v = match *p {
                        TypeTok::Int => self.gen_int_expr(depth + 1),
                        TypeTok::Float => self.gen_float_expr(depth + 1),
                        TypeTok::Bool => self.gen_bool_expr(depth + 1),
                        TypeTok::Str => self.gen_str_expr(depth + 1),
                        _ => todo!("[ERROR] {:?} is unsupported for parameters", *p),
                    };
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name.clone()), ast_params, Span::null_span())
            }

            _ => unreachable!(),
        };
        return val;
    }
    fn gen_bool_expr(&mut self, depth: usize) -> Ast {
        if depth > self.max_expr_depth {
            return Ast::BoolLit(self.rng.random_bool(0.5), Span::null_span());
        }
        let val = match self.rng.random_range(0..=7) {
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
                let mut ast_params: Vec<Ast> = vec![];
                for p in params {
                    let v = match *p {
                        TypeTok::Int => self.gen_int_expr(depth + 1),
                        TypeTok::Float => self.gen_float_expr(depth + 1),
                        TypeTok::Bool => self.gen_bool_expr(depth + 1),
                        TypeTok::Str => self.gen_str_expr(depth + 1),
                        _ => todo!("[ERROR] {:?} is unsupported for parameters", *p),
                    };
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name.clone()), ast_params, Span::null_span())
            }
            //arrays are hard because you have to make sure the access is in bound
            //functions are todo
            _ => unreachable!(),
        };
        return val;
    }
    fn gen_if_stmt(&mut self, stmt_depth: usize) -> Ast {
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
            stmts.push(self.gen_stmt(stmt_depth + 1));
        }
        self.scopes.pop();

        let else_stmts = if self.rng.random_bool(0.5) {
            self.scopes.push(Scope {
                vars: HashMap::new(),
            });
            let mut else_stmts = vec![];
            for _ in 0..block_len {
                else_stmts.push(self.gen_stmt(stmt_depth + 1));
            }
            self.scopes.pop();
            Some(else_stmts)
        } else {
            None
        };

        return Ast::IfStmt(Box::new(expr), stmts, else_stmts, Span::null_span());
    }
    fn gen_struct_expr(&mut self, depth: usize) -> (Ast, TypeTok) {
        //make a struct interface - right now each struct has its own interface
        let field_count = self.rng.random_range(1..=5);

        let mut field_types: BTreeMap<String, TypeTok> = BTreeMap::new();

        for _ in 0..field_count {
            let field_name = Alphabetic.sample_string(&mut self.rng, 10);
            field_types.insert(field_name, self._random_type());
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

        for (n, t) in &field_types {
            let v = match t {
                TypeTok::Int => self.gen_int_expr(depth + 1),
                TypeTok::Bool => self.gen_bool_expr(depth + 1),
                TypeTok::Float => self.gen_float_expr(depth + 1),
                TypeTok::Str => self.gen_str_expr(depth + 1),
                _ => todo!("{:?} is not supported for struct fields yet", t),
            };

            fields.insert(n.clone(), (v, t.clone()));
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
    fn gen_while_stmt(&mut self, stmt_depth: usize) -> Ast {
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
            stmts.push(self.gen_stmt(stmt_depth + 1));
        }
        self.scopes.pop();
        return Ast::WhileStmt(Box::new(expr), stmts, Span::null_span());
    }
    fn gen_function(&mut self) -> Ast {
        let param_count = self.rng.random_range(0..=4);
        let ret_type = self._random_type();
        let mut param_names: Vec<String> = vec![];
        for _ in 0..param_count {
            param_names.push(Alphabetic.sample_string(&mut self.rng, 10));
        }
        let mut param_types: Vec<TypeTok> = vec![];
        for _ in 0..param_count {
            param_types.push(self._random_type());
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
            body.push(self.gen_stmt(1));
        }
        self.scopes.pop();
        let ret = match ret_type {
            TypeTok::Int => self.gen_int_expr(0),
            TypeTok::Bool => self.gen_bool_expr(0),
            TypeTok::Float => self.gen_float_expr(0),
            TypeTok::Str => self.gen_str_expr(0),
            _ => todo!("{:?} is an invalid return type", ret_type),
        };
        body.push(ret);
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
    fn gen_stmt(&mut self, stmt_depth: usize) -> Ast {
        if stmt_depth == 0 {
            let f = self.gen_function();
            return f;
        }
        if stmt_depth > self.max_stmt_depth {
            return self.gen_var_dec(); //this is a bodge
        }
        return match self.rng.random_range(0..=2) {
            0 => self.gen_var_dec(),
            1 => self.gen_if_stmt(stmt_depth),
            2 => self.gen_while_stmt(stmt_depth),
            _ => unreachable!(),
        };
    }
    fn gen_int_expr(&mut self, depth: usize) -> Ast {
        if depth > self.max_expr_depth {
            return Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span());
        }
        let val = match self.rng.random_range(0..=5) {
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
                let mut ast_params: Vec<Ast> = vec![];
                for p in params {
                    let v = match *p {
                        TypeTok::Int => self.gen_int_expr(depth + 1),
                        TypeTok::Float => self.gen_float_expr(depth + 1),
                        TypeTok::Bool => self.gen_bool_expr(depth + 1),
                        TypeTok::Str => self.gen_str_expr(depth + 1),
                        _ => todo!("[ERROR] {:?} is unsupported for parameters", *p),
                    };
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name.clone()), ast_params, Span::null_span())
            }
            //arrays are hard because you have to make sure the access is in bound
            //functions are todo
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
        let val = match self.rng.random_range(0..=5) {
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
                let mut ast_params: Vec<Ast> = vec![];
                for p in params {
                    let v = match *p {
                        TypeTok::Int => self.gen_int_expr(depth + 1),
                        TypeTok::Float => self.gen_float_expr(depth + 1),
                        TypeTok::Bool => self.gen_bool_expr(depth + 1),
                        TypeTok::Str => self.gen_str_expr(depth + 1),
                        _ => todo!("[ERROR] {:?} is unsupported for parameters", *p),
                    };
                    ast_params.push(v);
                }
                Ast::FuncCall(Box::new(name.clone()), ast_params, Span::null_span())
            }
            //arrays are hard because you have to make sure the access is in bound
            //functions are todo
            _ => unreachable!(),
        };
        return val;
    }
    fn gen_expr(&mut self) -> (Ast, TypeTok) {
        let n = self.rng.random_range(0..=3);
        return match n {
            //should include structs and arrays
            0 => (self.gen_int_expr(0), TypeTok::Int),
            1 => (self.gen_float_expr(0), TypeTok::Float),
            2 => (self.gen_bool_expr(0), TypeTok::Bool),
            3 => self.gen_struct_expr(0),
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
            self.program.push(v);
        }
        self.interfaces.append(&mut self.program);
        return self.interfaces.clone();
    }
    fn gen_expr_of_type(&mut self, ty: TypeTok, depth: usize) -> Ast {
        match ty {
            TypeTok::Int => self.gen_int_expr(depth),
            TypeTok::Float => self.gen_float_expr(depth),
            TypeTok::Bool => self.gen_bool_expr(depth),
            TypeTok::Str => self.gen_str_expr(depth),
            _ => todo!("unsupported replacement type {:?}", ty),
        }
    }
    fn rewrite_expr(&mut self, expr: Ast, removed_name: &str, removed_ret: &TypeTok) -> Ast {
        match expr {
            Ast::FuncCall(name, args, span) => {
                if *name == removed_name {
                    return self.gen_expr_of_type(removed_ret.clone(), 0);
                }

                Ast::FuncCall(
                    name,
                    args.into_iter()
                        .map(|a| self.rewrite_expr(a, removed_name, removed_ret))
                        .collect(),
                    span,
                )
            }

            Ast::InfixExpr(lhs, rhs, op, span) => Ast::InfixExpr(
                Box::new(self.rewrite_expr(*lhs, removed_name, removed_ret)),
                Box::new(self.rewrite_expr(*rhs, removed_name, removed_ret)),
                op,
                span,
            ),

            Ast::EmptyExpr(expr, span) => Ast::EmptyExpr(
                Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                span,
            ),

            Ast::Not(expr, span) => Ast::Not(
                Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                span,
            ),

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

            Ast::VarDec(name, ty, expr, span) => Some(Ast::VarDec(
                name,
                ty,
                Box::new(self.rewrite_expr(*expr, removed_name, removed_ret)),
                span,
            )),

            Ast::IfStmt(cond, body, else_body, span) => {
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

            Ast::WhileStmt(cond, body, span) => {
                let cond = Box::new(self.rewrite_expr(*cond, removed_name, removed_ret));

                let body = body
                    .into_iter()
                    .filter_map(|s| self.rewrite_stmt(s, removed_name, removed_ret))
                    .collect();

                Some(Ast::WhileStmt(cond, body, span))
            }

            _ => Some(stmt),
        }
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

        let removed = funcs[self.rng.random_range(0..funcs.len())].clone();

        let (removed_name, removed_ret) = match removed {
            Ast::FuncDec(name, _, ret, _, _) => (*name, ret),
            _ => unreachable!(),
        };

        input
            .into_iter()
            .filter_map(|stmt| self.rewrite_stmt(stmt, &removed_name, &removed_ret))
            .collect()
    }
}
