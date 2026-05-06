use crate::errors::Span;
use crate::*;
use rand::RngExt;
use rand::{SeedableRng, rngs::StdRng};
use std::collections::HashMap;
use std::ops::RangeInclusive;
struct Scope {
    /// type -> Vec<VarNames>. The names are in no particular order, and one should be selected at random
    /// Also included here are any function parameters that are in scope
    vars: HashMap<TypeTok, Vec<String>>,
    /// type -> Vec<VarNames>. The names are in no particular order, and one should be selected at random
    /// Also included here are any function parameters that are in scope
    /// TypeTok is always TypeTok::Struct and must be searched to see if it contains the type needed
    struct_literals: HashMap<TypeTok, Vec<String>>,
}

pub struct TestRunner {
    ///stack of scopes, all starting with the root scope
    scopes: Vec<Scope>,
    ///the rng used for every bit of randomness in the fuzzer, if the test needs to be replayed, just save this rng
    rng: StdRng,
    rng_seed: u64,
    max_stmt_depth: usize,
    max_expr_depth: usize,
    program: Vec<Ast>,
    type_tok_range: RangeInclusive<usize>,
}

impl TestRunner {
    pub fn new() -> TestRunner {
        let root = Scope {
            vars: HashMap::new(),
            struct_literals: HashMap::new(),
        };
        let seed = 100u64; //this is sketchy
        return TestRunner {
            scopes: vec![root],
            rng: StdRng::seed_from_u64(seed),
            max_stmt_depth: 6,
            max_expr_depth: 3,
            program: vec![],
            rng_seed: seed,
            type_tok_range: 0..=2,
        };
    }
    fn _random_type(&mut self) -> TypeTok {
        return match self.rng.random_range(self.type_tok_range.clone()) {
            0 => TypeTok::Int,
            1 => TypeTok::Bool,
            2 => TypeTok::Float,
            _ => unreachable!(),
        };
    }
    fn _rand_int_infix_op(&mut self) -> InfixOp{
        return match self.rng.random_range(0..=4) {
            0 => InfixOp::Plus,
            1 => InfixOp::Minus,
            2 => InfixOp::Multiply,
            3 => InfixOp::Divide,
            4 => InfixOp::Modulo,
            _ => unreachable!()
        }
    }
    fn gen_int_expr(&mut self, depth: usize) -> Ast {

        if depth > self.max_expr_depth {
            return Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span());
        }
        let val = match self.rng.random_range(0..=4) {//for right now it does not do function calls
            0 => Ast::IntLit(self.rng.random_range(i64::MIN..i64::MAX), Span::null_span()),
            1 => Ast::InfixExpr(Box::new(self.gen_int_expr(depth + 1)), Box::new(self.gen_int_expr(depth + 1)), self._rand_int_infix_op(), Span::null_span()),
            2 => Ast::EmptyExpr(Box::new(self.gen_int_expr(depth + 1)), Span::null_span()),
            3 => {
                let candidate_variables: Vec<String> = self.scopes.iter()
                    .flat_map(|scope| scope.vars.get(&TypeTok::Int).into_iter().flatten())
                    .cloned()
                    .collect(); 
                let v = candidate_variables[self.rng.random_range(0..candidate_variables.len())].clone();

                Ast::VarRef(Box::new(v), Span::null_span())
            }
            4 => {
                //nested structs are currently not handled, that should be done in the future
                let candidate_variables: Vec<(String, String)> = self.scopes.iter()
                    .flat_map(|scope| scope.struct_literals.iter())
                    .filter_map(|(ty, names)| {
                        if let TypeTok::Struct(struct_ty) = ty {
                            Some((struct_ty, names))
                        } else {
                            None
                        }
                    })
                    .flat_map(|(struct_ty, names)| {
                        names.iter().flat_map(move |var_name| {
                            struct_ty.iter()
                                .filter(|(_, t)| ***t == TypeTok::Int)
                                .map(move |(field_name, _)| {
                                    (var_name.clone(), field_name.clone())
                                })
                        })
                    })
                    .collect();
                let (v, m) = candidate_variables[self.rng.random_range(0..candidate_variables.len())].clone();
                Ast::MemberAccess(Box::new(Ast::VarRef(Box::new(v), Span::null_span())), m, Span::null_span())
            }
            //arrays are hard because you have to make sure the access is in bound
            //functions are todo
            _ => unreachable!()
        };
        return val
    }
    fn gen_expr(&mut self) -> Ast {
        let ty = self._random_type();
        return match ty {
            TypeTok::Int => self.gen_int_expr(0),
            _ => todo!("{:?} is unreachable", ty)
        };
    }
    fn gen_var_dec(&mut self) -> Ast {

        return ();
    }
    pub fn generate(&mut self) -> Vec<Ast> {
        self.scopes.push(Scope {
            vars: HashMap::new(),
            struct_literals: HashMap::new()
        });
        return self.program.clone();
    }
}
