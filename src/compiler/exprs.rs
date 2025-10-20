use super::Compiler;
use super::Scope;
use crate::token::TypeTok;
use crate::parser::ast::{Ast, InfixOp};
use crate::debug;


use std::cell::RefCell;
use std::rc::Rc;

use cranelift::prelude::*;
use cranelift_module::{DataDescription};
use cranelift_module::{Module};

impl Compiler{
    pub fn compile_expr<M: Module>(
        &self,
        expr: &Ast,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> (Value, TypeTok) {
        debug!(targets: ["compiler", "compiler_verbose"], "in compile expression");
        if expr.node_type() != "IntLit"
            && expr.node_type() != "InfixExpr"
            && expr.node_type() != "VarRef"
            && expr.node_type() != "BoolLit"
            && expr.node_type() != "EmptyExpr"
            && expr.node_type() != "FuncCall"
            && expr.node_type() != "StringLit"
        {
            panic!("[ERROR] Unknown AST node type: {}", expr.node_type());
        }
    
        match expr {
            Ast::FuncCall(b_name, params) => {
                let name = *b_name.clone();
                let o_func = self.funcs.get(&name);
                if o_func.is_none() {
                    panic!("[ERROR] Function {} is undefined", name);
                }
                let (ret_type, id) = o_func.unwrap();
                let mut param_values: Vec<Value> = Vec::new();
                let mut last_type: TypeTok = TypeTok::Str;
                for p in params {
                    let (v, t) = self.compile_expr(p, _module, builder, scope);
                    last_type = t;
                    param_values.push(v);
                }
                if name == "print".to_string() || name == "println".to_string() {
                    //inject type params for print and println
                    if last_type == TypeTok::Str {
                        let v = builder.ins().iconst(types::I64, 0);
                        param_values.push(v);
                    } else if last_type == TypeTok::Bool {
                        let v = builder.ins().iconst(types::I64, 1);
                        param_values.push(v);
                    } else if last_type == TypeTok::Int {
                        let v = builder.ins().iconst(types::I64, 2);
                        param_values.push(v);
                    } else {
                        panic!(
                            "[ERROR] Cannot pase type {:?} to print or println",
                            last_type
                        );
                    }
                }
                let func_ref = _module.declare_func_in_func(id.clone(), builder.func);
    
                let call_inst = builder.ins().call(func_ref, &param_values.as_slice());
                let results = builder.inst_results(call_inst);
                if results.len() > 0 {
                    let ret_val = results[0];
    
                    return (ret_val, ret_type.clone());
                } else {
                    //This is a dummy, should not be sued
                    return (builder.ins().iconst(types::I64, 0), TypeTok::Void);
                }
            }
            Ast::IntLit(n) => (builder.ins().iconst(types::I64, *n), TypeTok::Int),
            Ast::BoolLit(b) => {
                let is_true: i64 = if *b { 1 } else { 0 };
                (builder.ins().iconst(types::I64, is_true), TypeTok::Bool)
            }
            Ast::StringLit(s) => {
                let data_id = _module
                    .declare_anonymous_data(false, false)
                    .expect("Failed to declare data");
    
                //Create null terminated string in mem
                let mut data_desc = DataDescription::new();
                let mut string_bytes = s.as_bytes().to_vec();
                string_bytes.push(0); // null terminator
                data_desc.define(string_bytes.into_boxed_slice());
                _module
                    .define_data(data_id, &data_desc)
                    .expect("Failed to define data");
    
                // Get a global value reference to the data
                let data_gv = _module.declare_data_in_func(data_id, builder.func);
                let string_ptr = builder.ins().global_value(types::I64, data_gv);
    
                // Call toy_malloc with the string pointer
                let malloc_func = self.funcs.get("malloc").expect("malloc not found");
                let func_ref = _module.declare_func_in_func(malloc_func.1, builder.func);
                let call_inst = builder.ins().call(func_ref, &[string_ptr]);
                let results = builder.inst_results(call_inst);
                let heap_ptr = results[0];
    
                (heap_ptr, TypeTok::Str)
            }
            Ast::EmptyExpr(child) => self.compile_expr(child, _module, builder, scope),
            Ast::InfixExpr(left, right, op) => {
                let (l, l_t) = self.compile_expr(left, _module, builder, scope);
                let (r, r_t) = self.compile_expr(right, _module, builder, scope);
                let l_type_str = l_t.type_str();
                let r_type_str = r_t.type_str();
    
                if l_type_str == "Int" && r_type_str == "Int" {
                    return match op {
                        InfixOp::Plus => (builder.ins().iadd(l, r), TypeTok::Int),
                        InfixOp::Minus => (builder.ins().isub(l, r), TypeTok::Int),
                        InfixOp::Multiply => (builder.ins().imul(l, r), TypeTok::Int),
                        InfixOp::Divide => (builder.ins().sdiv(l, r), TypeTok::Int),
                        InfixOp::Modulo => (builder.ins().srem(l, r), TypeTok::Int),
                        InfixOp::LessThan => {
                            let cmp = builder.ins().icmp(IntCC::SignedLessThan, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        InfixOp::LessThanEqt => {
                            let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        InfixOp::GreaterThan => {
                            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        InfixOp::GreaterThanEqt => {
                            let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        InfixOp::Equals => {
                            let cmp = builder.ins().icmp(IntCC::Equal, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        InfixOp::NotEquals => {
                            let cmp = builder.ins().icmp(IntCC::NotEqual, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        _ => panic!("[ERROR] Cant use operator {} on two ints", op),
                    };
                }
                if l_type_str == "Bool" && r_type_str == "Bool" {
                    return match op {
                        InfixOp::Equals => {
                            let cmp = builder.ins().icmp(IntCC::Equal, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        InfixOp::NotEquals => {
                            let cmp = builder.ins().icmp(IntCC::NotEqual, l, r);
                            (builder.ins().uextend(types::I64, cmp), TypeTok::Bool)
                        }
                        InfixOp::And => (builder.ins().band(l, r), TypeTok::Bool),
                        InfixOp::Or => (builder.ins().bor(l, r), TypeTok::Bool),
                        _ => panic!("[ERROR] Cant use operator {} on two bools", op),
                    };
                }
                if l_type_str == "Str" && r_type_str == "Str" {
                    match op {
                        InfixOp::Plus => {
                            let toy_concat = self.funcs.get("concat").expect("concat not found");
                            let func_ref = _module.declare_func_in_func(toy_concat.1, builder.func);
                            let call_inst = builder.ins().call(func_ref, &[l, r]);
                            let results = builder.inst_results(call_inst);
                            let heap_ptr = results[0];
                            return (heap_ptr, TypeTok::Str);
                        }
                        InfixOp::Equals => {
                            let toy_strequal =
                                self.funcs.get("strequal").expect("strequal not found");
                            let func_ref =
                                _module.declare_func_in_func(toy_strequal.1, builder.func);
                            let call_inst = builder.ins().call(func_ref, &[l, r]);
                            let results = builder.inst_results(call_inst);
                            let heap_ptr = results[0];
                            return (heap_ptr, TypeTok::Bool);
                        }
                        InfixOp::NotEquals => {
                            let toy_strequal =
                                self.funcs.get("strequal").expect("strequal not found");
                            let func_ref =
                                _module.declare_func_in_func(toy_strequal.1, builder.func);
                            let call_inst = builder.ins().call(func_ref, &[l, r]);
                            let results = builder.inst_results(call_inst);
                            let heap_ptr = results[0];
    
                            //inverts result of eq
                            let one = builder.ins().iconst(types::I64, 1);
                            let flipped = builder.ins().bxor(heap_ptr, one);
    
                            return (flipped, TypeTok::Bool);
                        }
                        _ => panic!(),
                    }
                }
    
                panic!(
                    "[ERROR] Unknown type combination, got l_type: {}, r_type: {}",
                    l_type_str, r_type_str
                );
            }
            Ast::VarRef(v) => {
                let v_string = v.clone();
                let (var, var_type) = scope.as_ref().borrow().get(*v_string);
                debug!(targets: ["compiler_verbose"], var);
                (builder.use_var(var), var_type.clone())
            }
    
            _ => todo!("Unknown expression type"),
        }
    }
}