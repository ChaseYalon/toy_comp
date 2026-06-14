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
        Ast::AnonFuncCall(callable, args, _) => {
            var_referenced_in(name, callable) || args.iter().any(|a| var_referenced_in(name, a))
        }
        // A lambda body cannot capture, so it never references an outer variable; recurse anyway
        // for robustness (a no-op in practice).
        Ast::LambdaDec(_, _, body, _) => body.iter().any(|s| var_referenced_in(name, s)),
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
    ///struct field-map -> registered interface name, so any Struct/StructArr value can be built
    ///against a real (already-emitted) StructInterface. Field map matches TypeTok::Struct's map.
    struct_interfaces: HashMap<BTreeMap<String, Box<TypeTok>>, String>,
    ///current lambda nesting depth, used to bound how deeply lambdas may nest in one another
    lambda_nesting: usize,
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
            prgm_length: 10,
            interfaces: vec![],
            functions: vec![],
            var_names_to_types: HashMap::new(),
            struct_interfaces: HashMap::new(),
            lambda_nesting: 0,
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
        match self.rng.random_range(0..=12) {
            0 => TypeTok::Int,
            1 => TypeTok::Bool,
            2 => TypeTok::Float,
            3 => TypeTok::Str,
            4 => TypeTok::IntArr(1),
            5 => TypeTok::BoolArr(1),
            6 => TypeTok::FloatArr(1),
            7 => TypeTok::StrArr(1),
            8 => TypeTok::StrArr(2),
            9 => TypeTok::IntArr(2),
            10 => TypeTok::BoolArr(2),
            11 => TypeTok::FloatArr(2),
            12 => self._random_lambda_type(),
            _ => unreachable!(),
        }
    }
    fn _random_return_type(&mut self) -> TypeTok {
        match self.rng.random_range(0..=11) {
            0 => TypeTok::Int,
            1 => TypeTok::Bool,
            2 => TypeTok::Float,
            3 => TypeTok::Str,
            4 => TypeTok::IntArr(1),
            5 => TypeTok::BoolArr(1),
            6 => TypeTok::FloatArr(1),
            7 => TypeTok::StrArr(1),
            8 => TypeTok::IntArr(2),
            9 => TypeTok::FloatArr(2),
            10 => TypeTok::StrArr(2),
            11 => self._random_lambda_type(),
            _ => unreachable!(),
        }
    }
    fn _rand_int_infix_op(&mut self) -> InfixOp {
        return match self.rng.random_range(0..=2) {
            0 => InfixOp::Plus,
            1 => InfixOp::Minus,
            2 => InfixOp::Multiply,
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
        let val = match self.rng.random_range(0..=7) {
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
            7 => match self.gen_anon_call(&TypeTok::Str, depth) {
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
        let val = match self.rng.random_range(0..=9) {
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
            9 => match self.gen_anon_call(&TypeTok::Bool, depth) {
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
    /// Registers a struct interface for `map` (or returns the existing name) and emits its
    /// StructInterface AST exactly once, so any StructLit referencing this field shape is valid.
    fn register_struct_interface(&mut self, map: BTreeMap<String, Box<TypeTok>>) -> String {
        if let Some(name) = self.struct_interfaces.get(&map) {
            return name.clone();
        }
        let name = Alphabetic.sample_string(&mut self.rng, 10);
        let unboxed: BTreeMap<String, TypeTok> =
            map.iter().map(|(k, v)| (k.clone(), (**v).clone())).collect();
        self.interfaces.push(Ast::StructInterface(
            Box::new(name.clone()),
            Box::new(unboxed),
            Span::null_span(),
        ));
        self.struct_interfaces.insert(map, name.clone());
        name
    }
    /// A field type for a struct. With a small depth budget it may itself be a (nested) struct —
    /// the encapsulator-inside-encapsulator case that stresses CTLA's deep-free. Kept to a single
    /// level of plain-struct nesting (no struct-array fields) so the compiler doesn't blow its
    /// stack on multiplicatively nested encapsulators.
    fn gen_field_type(&mut self, depth: usize) -> TypeTok {
        if depth < 1 && self.rng.random_range(0..=7) == 0 {
            let (_, map) = self.gen_struct_type(depth + 1);
            return TypeTok::Struct(map);
        }
        self._random_field_type()
    }
    /// Builds (and registers) a random struct interface, returning its name and field map.
    fn gen_struct_type(&mut self, depth: usize) -> (String, BTreeMap<String, Box<TypeTok>>) {
        let field_count = if depth > 0 {
            self.rng.random_range(1..=2)
        } else {
            self.rng.random_range(1..=4)
        };
        let mut map: BTreeMap<String, Box<TypeTok>> = BTreeMap::new();
        for _ in 0..field_count {
            let field_name = Alphabetic.sample_string(&mut self.rng, 10);
            let t = self.gen_field_type(depth);
            map.insert(field_name, Box::new(t));
        }
        let name = self.register_struct_interface(map.clone());
        (name, map)
    }
    /// Builds a StructLit matching `field_map` against the registered interface `interface_name`.
    fn gen_struct_lit(
        &mut self,
        interface_name: String,
        field_map: &BTreeMap<String, Box<TypeTok>>,
        depth: usize,
    ) -> Ast {
        let mut fields: BTreeMap<String, (Ast, TypeTok)> = BTreeMap::new();
        let entries: Vec<(String, TypeTok)> = field_map
            .iter()
            .map(|(k, v)| (k.clone(), (**v).clone()))
            .collect();
        for (n, t) in entries {
            let v = self.gen_arg_for_type(&t, depth + 1);
            fields.insert(n, (v, t));
        }
        Ast::StructLit(Box::new(interface_name), Box::new(fields), Span::null_span())
    }
    /// Builds a `dims`-dimensional array of structs matching `field_map`.
    fn gen_struct_arr_expr(
        &mut self,
        field_map: &BTreeMap<String, Box<TypeTok>>,
        dims: u64,
        depth: usize,
    ) -> Ast {
        let name = self.register_struct_interface(field_map.clone());
        let outer_len = self.rng.random_range(1..=2usize);
        let mut elements: Vec<Ast> = Vec::with_capacity(outer_len);
        for _ in 0..outer_len {
            if dims > 1 {
                elements.push(self.gen_struct_arr_expr(field_map, dims - 1, depth));
            } else {
                elements.push(self.gen_struct_lit(name.clone(), field_map, depth));
            }
        }
        Ast::ArrLit(
            TypeTok::StructArr(field_map.clone(), dims),
            elements,
            Span::null_span(),
        )
    }
    fn gen_struct_expr(&mut self, depth: usize) -> (Ast, TypeTok) {
        let (name, map) = self.gen_struct_type(depth);
        let lit = self.gen_struct_lit(name, &map, depth);
        (lit, TypeTok::Struct(map))
    }
    /// A scalar or 1D-array type usable in a lambda signature. Deliberately excludes structs and
    /// lambdas: a struct return would force interface registration that the delta-debug reducer
    /// (which runs in a fresh runner) can't reproduce, and nested-lambda signatures explode.
    fn _random_lambda_sig_type(&mut self) -> TypeTok {
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
    /// A random lambda type `(p0, p1, ...): ret`. The return may be Void.
    fn _random_lambda_type(&mut self) -> TypeTok {
        let param_count = self.rng.random_range(0..=2);
        let mut params = vec![];
        for _ in 0..param_count {
            params.push(self._random_lambda_sig_type());
        }
        let ret = if self.rng.random_range(0..=4) == 0 {
            TypeTok::Void
        } else {
            self._random_lambda_sig_type()
        };
        TypeTok::Lambda(params, Box::new(ret))
    }
    /// A recursion-free literal of a scalar or 1D/multi-dim array type. Used where generating a
    /// value must NOT recurse into function calls or lambdas (e.g. a too-deeply-nested lambda's
    /// return), since those paths are what blow the generator's stack.
    fn gen_literal_value(&mut self, ty: &TypeTok) -> Ast {
        match ty {
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
            TypeTok::IntArr(d)
            | TypeTok::BoolArr(d)
            | TypeTok::FloatArr(d)
            | TypeTok::StrArr(d) => {
                let elem = match ty {
                    TypeTok::IntArr(_) => TypeTok::Int,
                    TypeTok::BoolArr(_) => TypeTok::Bool,
                    TypeTok::FloatArr(_) => TypeTok::Float,
                    _ => TypeTok::Str,
                };
                let n = self.rng.random_range(1..=4usize);
                self.gen_nested_literal_arr(elem, *d, n)
            }
            _ => Ast::IntLit(0, Span::null_span()),
        }
    }
    /// Builds a lambda literal. The body is generated in a FRESH scope stack seeded only with the
    /// lambda's own parameters, so the body can never reference an outer variable — toy_lang
    /// lambdas do not capture, and emitting a capture would produce an invalid program.
    ///
    /// `lambda_nesting` is checked HERE (not just at the gen_expr call sites) because lambdas are
    /// also produced by gen_arg_for_type when building a lambda-typed function argument; without
    /// this guard a function that takes a lambda param and is called inside a lambda body recurses
    /// without bound. When too deep, the body is empty and the return is a plain literal so no
    /// further function calls / lambdas are generated.
    fn gen_lambda_dec(&mut self, param_types: Vec<TypeTok>, ret: TypeTok) -> Ast {
        let mut params: Vec<Ast> = vec![];
        let mut param_scope = Scope {
            vars: HashMap::new(),
        };
        for pt in &param_types {
            let n = Alphabetic.sample_string(&mut self.rng, 10);
            params.push(Ast::FuncParam(
                Box::new(n.clone()),
                pt.clone(),
                Span::null_span(),
            ));
            param_scope
                .vars
                .entry(pt.clone())
                .or_insert_with(Vec::new)
                .push(n);
        }
        let too_deep = self.lambda_nesting >= 2;
        self.lambda_nesting += 1;
        let saved = std::mem::replace(&mut self.scopes, vec![param_scope]);
        let mut body: Vec<Ast> = vec![];
        if !too_deep {
            let stmt_count = self.rng.random_range(0..=3);
            for _ in 0..stmt_count {
                body.extend(self.gen_stmt(1));
            }
        }
        if ret != TypeTok::Void {
            let r = if too_deep {
                self.gen_literal_value(&ret)
            } else {
                self.gen_arg_for_type(&ret, 0)
            };
            body.push(Ast::Return(Box::new(r), Span::null_span()));
        }
        self.scopes = saved;
        self.lambda_nesting -= 1;
        Ast::LambdaDec(params, ret, body, Span::null_span())
    }
    /// A lambda with no body and a literal return — capture-free, struct-free, and
    /// recursion-free. Used wherever generating a lambda must not pull in further structs/calls
    /// (top-level call args and reducer regeneration), so no un-emitted interfaces can leak.
    fn gen_trivial_lambda(&mut self, param_types: Vec<TypeTok>, ret: TypeTok) -> Ast {
        let params: Vec<Ast> = param_types
            .iter()
            .map(|pt| {
                Ast::FuncParam(
                    Box::new(Alphabetic.sample_string(&mut self.rng, 10)),
                    pt.clone(),
                    Span::null_span(),
                )
            })
            .collect();
        let mut body: Vec<Ast> = vec![];
        if ret != TypeTok::Void {
            let r = self.gen_literal_value(&ret);
            body.push(Ast::Return(Box::new(r), Span::null_span()));
        }
        Ast::LambdaDec(params, ret, body, Span::null_span())
    }
    /// Builds a `dims`-dimensional array of lambdas with the given signature.
    fn gen_lambda_arr_expr(
        &mut self,
        param_types: &[TypeTok],
        ret: &TypeTok,
        dims: u64,
    ) -> Ast {
        let outer_len = self.rng.random_range(1..=4usize);
        let mut elements: Vec<Ast> = Vec::with_capacity(outer_len);
        for _ in 0..outer_len {
            if dims > 1 {
                elements.push(self.gen_lambda_arr_expr(param_types, ret, dims - 1));
            } else {
                elements.push(self.gen_lambda_dec(param_types.to_vec(), ret.clone()));
            }
        }
        Ast::ArrLit(
            TypeTok::LambdaArr(param_types.to_vec(), Box::new(ret.clone()), dims),
            elements,
            Span::null_span(),
        )
    }
    /// If an in-scope lambda variable returns `ret_type`, builds an AnonFuncCall invoking it.
    /// This surfaces a heap-allocated callable being called across an alias.
    fn gen_anon_call(&mut self, ret_type: &TypeTok, depth: usize) -> Option<Ast> {
        let candidates: Vec<(Vec<TypeTok>, String)> = self
            .scopes
            .iter()
            .flat_map(|s| s.vars.iter())
            .filter_map(|(ty, names)| match ty {
                TypeTok::Lambda(params, r) if r.as_ref() == ret_type => Some(
                    names
                        .iter()
                        .map(|n| (params.clone(), n.clone()))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let (params, name) = candidates[self.rng.random_range(0..candidates.len())].clone();
        let mut args: Vec<Ast> = vec![];
        for p in &params {
            args.push(self.gen_arg_for_type(p, depth + 1));
        }
        Some(Ast::AnonFuncCall(
            Box::new(Ast::VarRef(Box::new(name), Span::null_span())),
            args,
            Span::null_span(),
        ))
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
        // Occasionally insert a `continue` somewhere in the body. It bumps the loop counter BEFORE
        // continuing so termination still holds (the trailing guard also bumps on the other path),
        // and it sits before the guard so the reducer's last-statement guard detection is intact.
        // Memory relevance: a `continue` skips allocations made later in the body on some paths,
        // exercising CTLA's "free along every control path" requirement.
        if self.rng.random_bool(0.3) {
            let cont_cond = self.gen_bool_expr(0);
            let bump = Ast::Assignment(
                Box::new(Ast::VarRef(Box::new(name.clone()), Span::null_span())),
                Box::new(Ast::InfixExpr(
                    Box::new(Ast::VarRef(Box::new(name.clone()), Span::null_span())),
                    Box::new(Ast::IntLit(1, Span::null_span())),
                    InfixOp::Plus,
                    Span::null_span(),
                )),
                Span::null_span(),
            );
            let cont_block = Ast::IfStmt(
                Box::new(cont_cond),
                vec![bump, Ast::Continue(Span::null_span())],
                None,
                Span::null_span(),
            );
            let pos = self.rng.random_range(0..=stmts.len());
            stmts.insert(pos, cont_block);
        }
        let if_stmt = Ast::IfStmt(
            Box::new(Ast::InfixExpr(
                Box::new(Ast::VarRef(Box::new(name.clone()), Span::null_span())),
                Box::new(Ast::IntLit(5, Span::null_span())),
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
        for _ in 0..self.rng.random_range(0..=5) {
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
        let val = match self.rng.random_range(0..=7) {
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
            7 => match self.gen_anon_call(&TypeTok::Int, depth) {
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
        let val = match self.rng.random_range(0..=7) {
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
            7 => match self.gen_anon_call(&TypeTok::Float, depth) {
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
        // Most values are cheap scalars / small 1D arrays, so a typical program stays small and
        // compiles fast. The allocation-heavy encapsulator features (structs, struct arrays,
        // multi-dimensional arrays, lambdas) are generated only occasionally — frequent enough for
        // coverage, rare enough that CTLA's per-allocation analysis doesn't explode the runtime.
        if self.rng.random_range(0..=5) != 0 {
            return match self.rng.random_range(0..=6) {
                0 => (self.gen_int_expr(0), TypeTok::Int),
                1 => (self.gen_float_expr(0), TypeTok::Float),
                2 => (self.gen_bool_expr(0), TypeTok::Bool),
                3 => {
                    let len = self.rng.random_range(1..=8usize);
                    (self.gen_arr_expr(TypeTok::Int, len), TypeTok::IntArr(1))
                }
                4 => {
                    let len = self.rng.random_range(1..=8usize);
                    (self.gen_arr_expr(TypeTok::Bool, len), TypeTok::BoolArr(1))
                }
                5 => {
                    let len = self.rng.random_range(1..=8usize);
                    (self.gen_arr_expr(TypeTok::Float, len), TypeTok::FloatArr(1))
                }
                6 => {
                    let len = self.rng.random_range(1..=8usize);
                    (self.gen_arr_expr(TypeTok::Str, len), TypeTok::StrArr(1))
                }
                _ => unreachable!(),
            };
        }
        // Allocation-heavy encapsulator features.
        match self.rng.random_range(0..=5) {
            0 => self.gen_struct_expr(0),
            1 => {
                let len = self.rng.random_range(1..=3usize);
                (self.gen_nested_str_arr_expr(len), TypeTok::StrArr(2))
            }
            2 => {
                // Multi-dimensional array. Memory-relevant because the outer array OWNS its inner
                // arrays (which are heap allocations), so deep-free must reclaim each one.
                let elem = match self.rng.random_range(0..=3) {
                    0 => TypeTok::Int,
                    1 => TypeTok::Bool,
                    2 => TypeTok::Float,
                    _ => TypeTok::Str,
                };
                let dims = if self.rng.random_range(0..=4) == 0 { 3 } else { 2 };
                let outer_len = self.rng.random_range(1..=2usize);
                let ty = Self::arr_type_of(&elem, dims);
                (self.gen_nested_arr_expr(elem, dims, outer_len), ty)
            }
            3 => {
                // Array of structs: the array OWNS each struct element (heap), and each struct may
                // own further heap children — deep nesting of encapsulators for CTLA to reclaim.
                let (_, map) = self.gen_struct_type(0);
                let arr = self.gen_struct_arr_expr(&map, 1, 0);
                (arr, TypeTok::StructArr(map, 1))
            }
            4 => {
                // A lambda value: a heap-allocated callable bound to a variable.
                if self.lambda_nesting >= 2 {
                    return (self.gen_int_expr(0), TypeTok::Int);
                }
                let lam_ty = self._random_lambda_type();
                let (params, ret) = match &lam_ty {
                    TypeTok::Lambda(p, r) => (p.clone(), (**r).clone()),
                    _ => unreachable!(),
                };
                let lit = self.gen_lambda_dec(params, ret);
                (lit, lam_ty)
            }
            5 => {
                // An array of lambdas: heap callables owned by an array encapsulator.
                if self.lambda_nesting >= 2 {
                    return (self.gen_int_expr(0), TypeTok::Int);
                }
                let inner = self._random_lambda_type();
                let (params, ret) = match &inner {
                    TypeTok::Lambda(p, r) => (p.clone(), (**r).clone()),
                    _ => unreachable!(),
                };
                let arr = self.gen_lambda_arr_expr(&params, &ret, 1);
                (arr, TypeTok::LambdaArr(params, Box::new(ret), 1))
            }
            _ => unreachable!(),
        }
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
        // Generate the top-level calls FIRST: building their arguments (e.g. a lambda passed to a
        // lambda-param function, whose body may declare structs) can register new struct
        // interfaces. Those must be collected before we snapshot self.interfaces below, or the
        // program will reference a StructInterface that was never emitted.
        let mut calls: Vec<Ast> = vec![];
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
            calls.push(Ast::FuncCall(Box::new(func_name), params, Span::null_span()));
        }

        self.interfaces.append(&mut self.program);
        let mut result = vec![Ast::ImportStmt("std.fuzz".to_string(), Span::null_span())];
        result.extend(self.interfaces.iter().cloned());
        result.extend(calls);

        return result;
    }
    fn gen_expr_of_type(&mut self, ty: TypeTok, depth: usize) -> Ast {
        match ty {
            TypeTok::Int => self.gen_int_expr(depth),
            TypeTok::Float => self.gen_float_expr(depth),
            TypeTok::Bool => self.gen_bool_expr(depth),
            TypeTok::Str => self.gen_str_expr(depth),
            TypeTok::IntArr(d)
            | TypeTok::BoolArr(d)
            | TypeTok::FloatArr(d)
            | TypeTok::StrArr(d) => {
                let elem = match ty {
                    TypeTok::IntArr(_) => TypeTok::Int,
                    TypeTok::BoolArr(_) => TypeTok::Bool,
                    TypeTok::FloatArr(_) => TypeTok::Float,
                    TypeTok::StrArr(_) => TypeTok::Str,
                    _ => unreachable!(),
                };
                let max = if d > 1 { 3 } else { 8 };
                let len = self.rng.random_range(1..=max);
                self.gen_nested_arr_expr(elem, d, len)
            }
            TypeTok::Struct(map) => {
                let name = self.register_struct_interface(map.clone());
                self.gen_struct_lit(name, &map, depth)
            }
            TypeTok::StructArr(map, dims) => self.gen_struct_arr_expr(&map, dims, depth),
            // Use TRIVIAL lambdas here (empty body, literal return). gen_expr_of_type fills
            // top-level call arguments and is the reducer's regeneration entry point; a rich body
            // could declare structs whose interfaces wouldn't be emitted into the reduced program.
            TypeTok::Lambda(params, ret) => self.gen_trivial_lambda(params, *ret),
            TypeTok::LambdaArr(params, ret, dims) => {
                let outer = self.rng.random_range(1..=3usize);
                let mut elems = Vec::with_capacity(outer);
                for _ in 0..outer {
                    if dims > 1 {
                        elems.push(self.gen_expr_of_type(
                            TypeTok::LambdaArr(params.clone(), ret.clone(), dims - 1),
                            depth,
                        ));
                    } else {
                        elems.push(self.gen_trivial_lambda(params.clone(), (*ret).clone()));
                    }
                }
                Ast::ArrLit(TypeTok::LambdaArr(params, ret, dims), elems, Span::null_span())
            }
            _ => todo!("unsupported replacement type {:?}", ty),
        }
    }
    fn typeof_node(&self, node: &Ast) -> Option<TypeTok> {
        return match node {
            Ast::IntLit(_, _) => Some(TypeTok::Int),
            Ast::FloatLit(_, _) => Some(TypeTok::Float),
            Ast::BoolLit(_, _) => Some(TypeTok::Bool),
            Ast::StringLit(_, _) => Some(TypeTok::Str),
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
                } else if op == &InfixOp::Plus
                    && (self.typeof_node(&(**l)) == Some(TypeTok::Str)
                        || self.typeof_node(&(**r)) == Some(TypeTok::Str))
                {
                    Some(TypeTok::Str)
                } else if self.typeof_node(&(**l)) == Some(TypeTok::Float)
                    || self.typeof_node(&(**r)) == Some(TypeTok::Float)
                {
                    Some(TypeTok::Float)
                } else {
                    Some(TypeTok::Int)
                }
            }
            Ast::EmptyExpr(sub, _) => self.typeof_node(&(**sub)),
            Ast::Not(_, _) => Some(TypeTok::Bool),
            Ast::VarDec(_, t, _, _) => {
                if matches!(t, TypeTok::Int | TypeTok::Float | TypeTok::Bool | TypeTok::Str) {
                    Some(t.clone())
                } else {
                    None
                }
            }
            Ast::VarRef(n, _) => {
                let res = self.var_names_to_types.get(&(**n).clone());
                if let Some(ty) = res {
                    match ty {
                        TypeTok::Int | TypeTok::Float | TypeTok::Bool | TypeTok::Str => {
                            Some(ty.clone())
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Ast::LambdaDec(params, ret, _, _) => {
                let param_types: Vec<TypeTok> = params
                    .iter()
                    .filter_map(|p| match p {
                        Ast::FuncParam(_, t, _) => Some(t.clone()),
                        _ => None,
                    })
                    .collect();
                Some(TypeTok::Lambda(param_types, Box::new(ret.clone())))
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
                    } else if op == InfixOp::Plus
                        && (self.typeof_node(&*lhs) == Some(TypeTok::Str)
                            || self.typeof_node(&*rhs) == Some(TypeTok::Str))
                    {
                        Ast::StringLit(
                            Box::new(Alphabetic.sample_string(&mut self.rng, 5)),
                            Span::null_span(),
                        )
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
                    let new_lhs = self.rewrite_expr(*lhs, removed_name, removed_ret);
                    let new_rhs = self.rewrite_expr(*rhs, removed_name, removed_ret);
                    // For Plus, both operands must be the same kind (str or numeric).
                    if op == InfixOp::Plus {
                        let lhs_str = self.typeof_node(&new_lhs) == Some(TypeTok::Str);
                        let rhs_str = self.typeof_node(&new_rhs) == Some(TypeTok::Str);
                        if lhs_str && !rhs_str {
                            let fixed =
                                self.gen_expr_of_type(TypeTok::Str, self.max_expr_depth + 1);
                            return Ast::InfixExpr(Box::new(new_lhs), Box::new(fixed), op, span);
                        } else if rhs_str && !lhs_str {
                            let fixed =
                                self.gen_expr_of_type(TypeTok::Str, self.max_expr_depth + 1);
                            return Ast::InfixExpr(Box::new(fixed), Box::new(new_rhs), op, span);
                        }
                    }
                    Ast::InfixExpr(Box::new(new_lhs), Box::new(new_rhs), op, span)
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
                let expected_elem_ty: Option<TypeTok> = match &ty {
                    TypeTok::IntArr(1) => Some(TypeTok::Int),
                    TypeTok::FloatArr(1) => Some(TypeTok::Float),
                    TypeTok::BoolArr(1) => Some(TypeTok::Bool),
                    TypeTok::StrArr(1) => Some(TypeTok::Str),
                    TypeTok::StrArr(2) => Some(TypeTok::StrArr(1)),
                    _ => None,
                };
                let mut new_elems: Vec<Ast> = elems
                    .into_iter()
                    .map(|e| {
                        let rewritten = self.rewrite_expr(e, removed_name, removed_ret);
                        if let Some(ref expected) = expected_elem_ty {
                            if let Some(actual) = self.typeof_node(&rewritten) {
                                if &actual != expected {
                                    return self
                                        .gen_arg_for_type(expected, self.max_expr_depth + 1);
                                }
                            }
                        }
                        rewritten
                    })
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

            Ast::AnonFuncCall(callable, args, span) => Ast::AnonFuncCall(
                Box::new(self.rewrite_expr(*callable, removed_name, removed_ret)),
                args.into_iter()
                    .map(|a| self.rewrite_expr(a, removed_name, removed_ret))
                    .collect(),
                span,
            ),

            // Rewrite inside the lambda body. The body cannot reference outer state, and the
            // reducer's regenerated expressions are capture-free literals, so this stays valid.
            Ast::LambdaDec(params, ret, body, span) => {
                let new_body = body
                    .into_iter()
                    .filter_map(|s| self.rewrite_stmt(s, removed_name, removed_ret))
                    .collect();
                Ast::LambdaDec(params, ret, new_body, span)
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

                let mut body: Vec<Ast> = body
                    .into_iter()
                    .filter_map(|s| self.rewrite_stmt(s, removed_name, removed_ret))
                    .collect();

                for stmt in &mut body {
                    if let Ast::Return(expr, _) = stmt {
                        if let Some(actual_ty) = self.typeof_node(expr) {
                            if actual_ty != ret {
                                *expr = Box::new(
                                    self.gen_expr_of_type(ret.clone(), self.max_expr_depth + 1),
                                );
                            }
                        }
                    }
                }

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
                if body.len() == 0 {
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
                if body.len() == 0 {
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
                | TypeTok::IntArr(2)
                | TypeTok::BoolArr(2)
                | TypeTok::FloatArr(2)
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
            TypeTok::IntArr(2) => TypeTok::IntArr(1),
            TypeTok::BoolArr(2) => TypeTok::BoolArr(1),
            TypeTok::FloatArr(2) => TypeTok::FloatArr(1),
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
        self.gen_nested_arr_expr(TypeTok::Str, 2, outer_len)
    }
    /// Maps a scalar element type + dimension count to the matching array TypeTok.
    fn arr_type_of(elem_type: &TypeTok, dims: u64) -> TypeTok {
        match elem_type {
            TypeTok::Int => TypeTok::IntArr(dims),
            TypeTok::Bool => TypeTok::BoolArr(dims),
            TypeTok::Float => TypeTok::FloatArr(dims),
            TypeTok::Str => TypeTok::StrArr(dims),
            _ => unreachable!(),
        }
    }
    /// Builds an `dims`-dimensional array literal of a scalar element type. Inner elements use
    /// `gen_arr_expr` (which may reference in-scope vars/function calls), so this is for var
    /// declarations rather than argument positions.
    fn gen_nested_arr_expr(&mut self, elem_type: TypeTok, dims: u64, outer_len: usize) -> Ast {
        if dims <= 1 {
            return self.gen_arr_expr(elem_type, outer_len);
        }
        let mut elements: Vec<Ast> = Vec::with_capacity(outer_len);
        for _ in 0..outer_len {
            let inner_len = self.rng.random_range(1..=3usize);
            elements.push(self.gen_nested_arr_expr(elem_type.clone(), dims - 1, inner_len));
        }
        Ast::ArrLit(Self::arr_type_of(&elem_type, dims), elements, Span::null_span())
    }
    /// Like `gen_nested_arr_expr` but uses literal-only inner elements (`gen_literal_arr`); safe for
    /// argument / element positions where referencing in-scope state could change the value's type.
    fn gen_nested_literal_arr(&mut self, elem_type: TypeTok, dims: u64, outer_len: usize) -> Ast {
        if dims <= 1 {
            return self.gen_literal_arr(elem_type, outer_len);
        }
        let mut elements: Vec<Ast> = Vec::with_capacity(outer_len);
        for _ in 0..outer_len {
            let inner_len = self.rng.random_range(1..=3usize);
            elements.push(self.gen_nested_literal_arr(elem_type.clone(), dims - 1, inner_len));
        }
        Ast::ArrLit(Self::arr_type_of(&elem_type, dims), elements, Span::null_span())
    }
    /// Generates an expression of any type, including array types, for use as a function argument
    /// or array element. Prefers referencing in-scope variables over generating fresh literals.
    fn gen_arg_for_type(&mut self, ty: &TypeTok, depth: usize) -> Ast {
        match ty {
            TypeTok::Int => self.gen_int_expr(depth),
            TypeTok::Float => self.gen_float_expr(depth),
            TypeTok::Bool => self.gen_bool_expr(depth),
            TypeTok::Str => self.gen_str_expr(depth),
            TypeTok::IntArr(d)
            | TypeTok::BoolArr(d)
            | TypeTok::FloatArr(d)
            | TypeTok::StrArr(d) => {
                let dims = *d;
                let elem = match ty {
                    TypeTok::IntArr(_) => TypeTok::Int,
                    TypeTok::BoolArr(_) => TypeTok::Bool,
                    TypeTok::FloatArr(_) => TypeTok::Float,
                    TypeTok::StrArr(_) => TypeTok::Str,
                    _ => unreachable!(),
                };
                // Prefer an in-scope variable of the exact array type.
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(ty).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                // Or surface a row from a higher-dimensional array in scope (alias/escape stress).
                if let Some(expr) = self.gen_rand_read(ty) {
                    return expr;
                }
                let max = if dims > 1 { 3 } else { 6 };
                let n = self.rng.random_range(1..=max);
                self.gen_nested_literal_arr(elem, dims, n)
            }
            TypeTok::Struct(map) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(ty).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                let name = self.register_struct_interface(map.clone());
                self.gen_struct_lit(name, map, depth)
            }
            TypeTok::StructArr(map, dims) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(ty).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                self.gen_struct_arr_expr(map, *dims, depth)
            }
            TypeTok::Lambda(params, ret) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(ty).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                self.gen_lambda_dec(params.clone(), (**ret).clone())
            }
            TypeTok::LambdaArr(params, ret, dims) => {
                let candidates: Vec<String> = self
                    .scopes
                    .iter()
                    .flat_map(|s| s.vars.get(ty).into_iter().flatten())
                    .cloned()
                    .collect();
                if !candidates.is_empty() && self.rng.random_bool(0.6) {
                    let v = candidates[self.rng.random_range(0..candidates.len())].clone();
                    return Ast::VarRef(Box::new(v), Span::null_span());
                }
                self.gen_lambda_arr_expr(params, ret, *dims)
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
        // Only element types that have a matching std.fuzz::read_rand overload (scalar elements →
        // 1D arrays, and 1D-array elements → 2D arrays). Deeper reads have no overload.
        let arr_type = match elem_type {
            TypeTok::Int => TypeTok::IntArr(1),
            TypeTok::Bool => TypeTok::BoolArr(1),
            TypeTok::Float => TypeTok::FloatArr(1),
            TypeTok::Str => TypeTok::StrArr(1),
            TypeTok::IntArr(1) => TypeTok::IntArr(2),
            TypeTok::BoolArr(1) => TypeTok::BoolArr(2),
            TypeTok::FloatArr(1) => TypeTok::FloatArr(2),
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

    fn replace_varref_in_expr(expr: Ast, name: &str, replacement: &Ast) -> Ast {
        match expr {
            Ast::VarRef(n, _) if *n == name => replacement.clone(),
            Ast::InfixExpr(l, r, op, span) => Ast::InfixExpr(
                Box::new(Self::replace_varref_in_expr(*l, name, replacement)),
                Box::new(Self::replace_varref_in_expr(*r, name, replacement)),
                op,
                span,
            ),
            Ast::EmptyExpr(e, span) => Ast::EmptyExpr(
                Box::new(Self::replace_varref_in_expr(*e, name, replacement)),
                span,
            ),
            Ast::Not(e, span) => Ast::Not(
                Box::new(Self::replace_varref_in_expr(*e, name, replacement)),
                span,
            ),
            Ast::FuncCall(fname, args, span) => Ast::FuncCall(
                fname,
                args.into_iter()
                    .map(|a| Self::replace_varref_in_expr(a, name, replacement))
                    .collect(),
                span,
            ),
            Ast::ArrLit(ty, elems, span) => Ast::ArrLit(
                ty,
                elems
                    .into_iter()
                    .map(|e| Self::replace_varref_in_expr(e, name, replacement))
                    .collect(),
                span,
            ),
            Ast::MemberAccess(e, field, span) => Ast::MemberAccess(
                Box::new(Self::replace_varref_in_expr(*e, name, replacement)),
                field,
                span,
            ),
            Ast::IndexAccess(e, idx, span) => Ast::IndexAccess(
                Box::new(Self::replace_varref_in_expr(*e, name, replacement)),
                Box::new(Self::replace_varref_in_expr(*idx, name, replacement)),
                span,
            ),
            Ast::StructLit(sname, fields, span) => {
                let new_fields = fields
                    .into_iter()
                    .map(|(k, (e, ty))| (k, (Self::replace_varref_in_expr(e, name, replacement), ty)))
                    .collect();
                Ast::StructLit(sname, Box::new(new_fields), span)
            }
            Ast::AnonFuncCall(callable, args, span) => Ast::AnonFuncCall(
                Box::new(Self::replace_varref_in_expr(*callable, name, replacement)),
                args.into_iter()
                    .map(|a| Self::replace_varref_in_expr(a, name, replacement))
                    .collect(),
                span,
            ),
            Ast::LambdaDec(params, ret, body, span) => Ast::LambdaDec(
                params,
                ret,
                body.into_iter()
                    .map(|s| Self::replace_varref_in_stmt(s, name, replacement))
                    .collect(),
                span,
            ),
            _ => expr,
        }
    }

    fn replace_varref_in_stmt(stmt: Ast, name: &str, replacement: &Ast) -> Ast {
        match stmt {
            Ast::VarDec(n, ty, expr, span) => Ast::VarDec(
                n,
                ty,
                Box::new(Self::replace_varref_in_expr(*expr, name, replacement)),
                span,
            ),
            Ast::Assignment(lhs, rhs, span) => Ast::Assignment(
                Box::new(Self::replace_varref_in_expr(*lhs, name, replacement)),
                Box::new(Self::replace_varref_in_expr(*rhs, name, replacement)),
                span,
            ),
            Ast::Return(expr, span) => Ast::Return(
                Box::new(Self::replace_varref_in_expr(*expr, name, replacement)),
                span,
            ),
            Ast::IfStmt(cond, body, else_body, span) => Ast::IfStmt(
                Box::new(Self::replace_varref_in_expr(*cond, name, replacement)),
                body.into_iter()
                    .map(|s| Self::replace_varref_in_stmt(s, name, replacement))
                    .collect(),
                else_body.map(|b| {
                    b.into_iter()
                        .map(|s| Self::replace_varref_in_stmt(s, name, replacement))
                        .collect()
                }),
                span,
            ),
            Ast::WhileStmt(cond, body, span) => Ast::WhileStmt(
                Box::new(Self::replace_varref_in_expr(*cond, name, replacement)),
                body.into_iter()
                    .map(|s| Self::replace_varref_in_stmt(s, name, replacement))
                    .collect(),
                span,
            ),
            Ast::FuncCall(fname, args, span) => Ast::FuncCall(
                fname,
                args.into_iter()
                    .map(|a| Self::replace_varref_in_expr(a, name, replacement))
                    .collect(),
                span,
            ),
            _ => stmt,
        }
    }

    fn remove_arg_from_expr(expr: Ast, func_name: &str, param_idx: usize) -> Ast {
        match expr {
            Ast::FuncCall(name, args, span) => {
                if *name == func_name {
                    let new_args = args
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| *i != param_idx)
                        .map(|(_, a)| a)
                        .collect();
                    Ast::FuncCall(name, new_args, span)
                } else {
                    Ast::FuncCall(
                        name,
                        args.into_iter()
                            .map(|a| Self::remove_arg_from_expr(a, func_name, param_idx))
                            .collect(),
                        span,
                    )
                }
            }
            Ast::InfixExpr(l, r, op, span) => Ast::InfixExpr(
                Box::new(Self::remove_arg_from_expr(*l, func_name, param_idx)),
                Box::new(Self::remove_arg_from_expr(*r, func_name, param_idx)),
                op,
                span,
            ),
            Ast::EmptyExpr(e, span) => Ast::EmptyExpr(
                Box::new(Self::remove_arg_from_expr(*e, func_name, param_idx)),
                span,
            ),
            Ast::Not(e, span) => Ast::Not(
                Box::new(Self::remove_arg_from_expr(*e, func_name, param_idx)),
                span,
            ),
            Ast::ArrLit(ty, elems, span) => Ast::ArrLit(
                ty,
                elems
                    .into_iter()
                    .map(|e| Self::remove_arg_from_expr(e, func_name, param_idx))
                    .collect(),
                span,
            ),
            Ast::MemberAccess(e, field, span) => Ast::MemberAccess(
                Box::new(Self::remove_arg_from_expr(*e, func_name, param_idx)),
                field,
                span,
            ),
            Ast::IndexAccess(e, idx, span) => Ast::IndexAccess(
                Box::new(Self::remove_arg_from_expr(*e, func_name, param_idx)),
                Box::new(Self::remove_arg_from_expr(*idx, func_name, param_idx)),
                span,
            ),
            Ast::StructLit(sname, fields, span) => {
                let new_fields = fields
                    .into_iter()
                    .map(|(k, (e, ty))| (k, (Self::remove_arg_from_expr(e, func_name, param_idx), ty)))
                    .collect();
                Ast::StructLit(sname, Box::new(new_fields), span)
            }
            Ast::AnonFuncCall(callable, args, span) => Ast::AnonFuncCall(
                Box::new(Self::remove_arg_from_expr(*callable, func_name, param_idx)),
                args.into_iter()
                    .map(|a| Self::remove_arg_from_expr(a, func_name, param_idx))
                    .collect(),
                span,
            ),
            Ast::LambdaDec(params, ret, body, span) => Ast::LambdaDec(
                params,
                ret,
                body.into_iter()
                    .map(|s| Self::remove_arg_from_stmt(s, func_name, param_idx))
                    .collect(),
                span,
            ),
            _ => expr,
        }
    }

    fn remove_arg_from_stmt(stmt: Ast, func_name: &str, param_idx: usize) -> Ast {
        match stmt {
            Ast::FuncDec(name, params, ret, body, span) => Ast::FuncDec(
                name,
                params,
                ret,
                body.into_iter()
                    .map(|s| Self::remove_arg_from_stmt(s, func_name, param_idx))
                    .collect(),
                span,
            ),
            Ast::VarDec(n, ty, expr, span) => Ast::VarDec(
                n,
                ty,
                Box::new(Self::remove_arg_from_expr(*expr, func_name, param_idx)),
                span,
            ),
            Ast::Assignment(lhs, rhs, span) => Ast::Assignment(
                Box::new(Self::remove_arg_from_expr(*lhs, func_name, param_idx)),
                Box::new(Self::remove_arg_from_expr(*rhs, func_name, param_idx)),
                span,
            ),
            Ast::Return(expr, span) => Ast::Return(
                Box::new(Self::remove_arg_from_expr(*expr, func_name, param_idx)),
                span,
            ),
            Ast::IfStmt(cond, body, else_body, span) => Ast::IfStmt(
                Box::new(Self::remove_arg_from_expr(*cond, func_name, param_idx)),
                body.into_iter()
                    .map(|s| Self::remove_arg_from_stmt(s, func_name, param_idx))
                    .collect(),
                else_body.map(|b| {
                    b.into_iter()
                        .map(|s| Self::remove_arg_from_stmt(s, func_name, param_idx))
                        .collect()
                }),
                span,
            ),
            Ast::WhileStmt(cond, body, span) => Ast::WhileStmt(
                Box::new(Self::remove_arg_from_expr(*cond, func_name, param_idx)),
                body.into_iter()
                    .map(|s| Self::remove_arg_from_stmt(s, func_name, param_idx))
                    .collect(),
                span,
            ),
            Ast::FuncCall(name, args, span) => {
                Self::remove_arg_from_expr(Ast::FuncCall(name, args, span), func_name, param_idx)
            }
            _ => stmt,
        }
    }

    /// Remove a random parameter from a random function declaration plus all call sites.
    /// If the parameter is referenced in the body, VarRef nodes are replaced with a fresh literal.
    pub fn reduce_params(&mut self, input: Vec<Ast>) -> Vec<Ast> {
        let func_indices: Vec<usize> = input
            .iter()
            .enumerate()
            .filter(|(_, n)| n.node_type() == "FuncDec")
            .map(|(i, _)| i)
            .collect();
        if func_indices.is_empty() {
            return input;
        }
        // Find functions that actually have parameters
        let candidates: Vec<usize> = func_indices
            .into_iter()
            .filter(|&i| {
                matches!(&input[i], Ast::FuncDec(_, params, _, _, _) if !params.is_empty())
            })
            .collect();
        if candidates.is_empty() {
            return input;
        }
        let func_idx = candidates[self.rng.random_range(0..candidates.len())];
        let (func_name, param_idx, param_name, param_type) = match &input[func_idx] {
            Ast::FuncDec(name, params, _, _, _) => {
                let pi = self.rng.random_range(0..params.len());
                match &params[pi] {
                    Ast::FuncParam(pname, ptype, _) => {
                        ((**name).clone(), pi, (**pname).clone(), ptype.clone())
                    }
                    _ => return input,
                }
            }
            _ => return input,
        };

        let replacement = self.gen_expr_of_type(param_type, self.max_expr_depth + 1);

        input
            .into_iter()
            .map(|stmt| {
                let stmt = Self::remove_arg_from_stmt(stmt, &func_name, param_idx);
                // For the target function, also remove the parameter from its signature
                // and replace body references with the generated literal
                match stmt {
                    Ast::FuncDec(name, mut params, ret, body, span)
                        if *name == func_name =>
                    {
                        params.remove(param_idx);
                        let body = body
                            .into_iter()
                            .map(|s| Self::replace_varref_in_stmt(s, &param_name, &replacement))
                            .collect();
                        Ast::FuncDec(name, params, ret, body, span)
                    }
                    other => other,
                }
            })
            .collect()
    }

    /// Remove a single top-level FuncCall (not inside any function body).
    /// Allows isolating which call triggers the crash without touching function definitions.
    pub fn reduce_top_level_call(&mut self, input: Vec<Ast>) -> Vec<Ast> {
        let call_indices: Vec<usize> = input
            .iter()
            .enumerate()
            .filter(|(_, n)| n.node_type() == "FuncCall")
            .map(|(i, _)| i)
            .collect();
        if call_indices.is_empty() {
            return input;
        }
        let idx = call_indices[self.rng.random_range(0..call_indices.len())];
        let mut result = input;
        result.remove(idx);
        result
    }

    /// Replace a random function's body with just `return <literal>` (or empty for void).
    /// Much faster than one-statement-at-a-time when function bodies are large.
    pub fn simplify_func_body(&mut self, input: Vec<Ast>) -> Vec<Ast> {
        let func_indices: Vec<usize> = input
            .iter()
            .enumerate()
            .filter(|(_, n)| n.node_type() == "FuncDec")
            .map(|(i, _)| i)
            .collect();
        if func_indices.is_empty() {
            return input;
        }
        let idx = func_indices[self.rng.random_range(0..func_indices.len())];
        let mut result = input;
        if let Ast::FuncDec(name, params, ret, body, span) = result[idx].clone() {
            let is_minimal = match &ret {
                TypeTok::Void => body.is_empty(),
                _ => body.len() == 1 && body.iter().any(|s| matches!(s, Ast::Return(..))),
            };
            if is_minimal {
                return result;
            }
            let new_body = if ret != TypeTok::Void {
                let ret_expr = self.gen_expr_of_type(ret.clone(), self.max_expr_depth + 1);
                vec![Ast::Return(Box::new(ret_expr), Span::null_span())]
            } else {
                vec![]
            };
            result[idx] = Ast::FuncDec(name, params, ret, new_body, span);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Generation must terminate without panicking (no unbounded struct/lambda recursion) and must
    /// always emit the std.fuzz import plus at least one node, for a spread of seeds. Cheap because
    /// it only builds the AST — it does not compile or run the program.
    #[test]
    fn test_generate_terminates_and_is_well_formed() {
        for seed in 0u64..40 {
            let mut runner = TestRunner::new_with_seed(seed);
            let prog = runner.generate();
            assert!(prog.len() >= 1, "seed {seed} produced an empty program");
            assert!(
                matches!(prog.first(), Some(Ast::ImportStmt(m, _)) if m == "std.fuzz"),
                "seed {seed} did not start with the std.fuzz import"
            );
        }
    }

    /// Every StructLit the generator emits must reference a StructInterface that is also emitted in
    /// the same program — the invariant that broke when interfaces were snapshotted before the
    /// top-level call arguments were generated.
    #[test]
    fn test_generated_struct_lits_have_declared_interfaces() {
        fn collect(node: &Ast, declared: &mut std::collections::HashSet<String>, used: &mut Vec<String>) {
            match node {
                Ast::StructInterface(n, _, _) => {
                    declared.insert((**n).clone());
                }
                Ast::StructLit(n, fields, _) => {
                    used.push((**n).clone());
                    for (_, (e, _)) in fields.iter() {
                        collect(e, declared, used);
                    }
                }
                Ast::FuncDec(_, _, _, body, _) => {
                    for s in body {
                        collect(s, declared, used);
                    }
                }
                Ast::LambdaDec(_, _, body, _) => {
                    for s in body {
                        collect(s, declared, used);
                    }
                }
                Ast::VarDec(_, _, e, _) => collect(e, declared, used),
                Ast::Return(e, _) => collect(e, declared, used),
                Ast::Assignment(l, r, _) => {
                    collect(l, declared, used);
                    collect(r, declared, used);
                }
                Ast::IfStmt(c, b, eb, _) => {
                    collect(c, declared, used);
                    for s in b {
                        collect(s, declared, used);
                    }
                    if let Some(eb) = eb {
                        for s in eb {
                            collect(s, declared, used);
                        }
                    }
                }
                Ast::WhileStmt(c, b, _) => {
                    collect(c, declared, used);
                    for s in b {
                        collect(s, declared, used);
                    }
                }
                Ast::FuncCall(_, args, _) | Ast::AnonFuncCall(_, args, _) => {
                    for a in args {
                        collect(a, declared, used);
                    }
                }
                Ast::ArrLit(_, elems, _) => {
                    for e in elems {
                        collect(e, declared, used);
                    }
                }
                Ast::InfixExpr(l, r, _, _) => {
                    collect(l, declared, used);
                    collect(r, declared, used);
                }
                Ast::Not(e, _) | Ast::EmptyExpr(e, _) | Ast::MemberAccess(e, _, _) => {
                    collect(e, declared, used)
                }
                Ast::IndexAccess(e, i, _) => {
                    collect(e, declared, used);
                    collect(i, declared, used);
                }
                _ => {}
            }
        }
        for seed in 0u64..40 {
            let mut runner = TestRunner::new_with_seed(seed);
            let prog = runner.generate();
            let mut declared = std::collections::HashSet::new();
            let mut used = Vec::new();
            for n in &prog {
                collect(n, &mut declared, &mut used);
            }
            for name in &used {
                assert!(
                    declared.contains(name),
                    "seed {seed}: StructLit references undeclared interface '{name}'"
                );
            }
        }
    }
}
