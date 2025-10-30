use super::Compiler;
use super::Scope;
use crate::debug;
use crate::parser::ast::{Ast, InfixOp};
use crate::token::TypeTok;
use std::cell::RefCell;
use std::rc::Rc;

use cranelift::prelude::*;
use cranelift_module::DataDescription;
use cranelift_module::Module;

impl Compiler {
    pub fn inject_type_param<M: Module>(
        &self,
        t: &TypeTok,
        inject_dimension: bool,
        _module: &M,
        builder: &mut FunctionBuilder<'_>,
        param_values: &mut Vec<Value>,
    ) {
      
        let (n, degree ) = match t {
            &TypeTok::Str => (0, 0),
            &TypeTok::Bool => (1, 0),
            &TypeTok::Int => (2, 0),
            &TypeTok::Float => (3, 0),
            &TypeTok::StrArr(n) => (4, n),
            &TypeTok::BoolArr(n) => (5, n),
            &TypeTok::IntArr(n) => (6, n),
            &TypeTok::FloatArr(n) => (7, n),
            _ => panic!("[ERROR] Cannot parse type {:?}", t)
        };
        let v = builder.ins().iconst(types::I64, n);
        param_values.push(v);
        if inject_dimension {

            let d = builder.ins().iconst(types::I64, degree as i64);
            param_values.push(d);
        }
    }
    pub fn compile_arr_lit<M: Module>(
        &self,
        arr: &Vec<Ast>,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>
    )-> Value{
        let mut arr_items: Vec<Value> = Vec::new();
        let mut arr_types: Vec<TypeTok> = Vec::new();
        for expr in arr {
            let (val, t) = self.compile_expr(expr, module, builder, scope);
            arr_items.push(val);
            arr_types.push(t);
        }
        let (_, arr_malloc_global) = self.funcs.get("toy_malloc_arr").unwrap();
        let (_, arr_write_global) = self.funcs.get("toy_write_to_arr").unwrap();
        let arr_malloc = module.declare_func_in_func(*arr_malloc_global, &mut builder.func);
        let arr_write = module.declare_func_in_func(*arr_write_global, &mut builder.func);
        
        //Calls toy malloc with the correct params
        let len = builder.ins().iconst(types::I64, arr_items.len() as i64);
        let mut params = [len].to_vec();
        self.inject_type_param(&arr_types[0], false, module, builder, &mut params);
        let call_res = builder.ins().call(arr_malloc, params.as_slice());
        let arr_ptr = builder.inst_results(call_res)[0];

        for (i, item) in arr_items.iter().enumerate(){
            let mut params = [arr_ptr, item.clone()].to_vec();
            let idx = builder.ins().iconst(types::I64, i as i64);
            params.push(idx);
            self.inject_type_param(&arr_types[i], false, module, builder, &mut params);
            builder.ins().call(arr_write, params.as_slice());
        }

        return arr_ptr;
    }
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
            && expr.node_type() != "FloatLit"
            && expr.node_type() != "ArrLit"
            && expr.node_type() != "ArrRef"
        {
            panic!("[ERROR] Unknown AST node type: {}", expr.node_type());
        }

        match expr {
            Ast::FuncCall(b_name, params) => {
                let mut name = *b_name.clone();
                let mut param_values: Vec<Value> = Vec::new();
                let mut last_type: TypeTok = TypeTok::Str;
                for p in params {
                    let (v, t) = self.compile_expr(p, _module, builder, scope);
                    last_type = t;
                    param_values.push(v);
                }
                if name == "len".to_string() {
                    if last_type == TypeTok::Str {
                        name = "strlen".to_string();
                    } else {
                        name = "arrlen".to_string();
                    }
                }
                let o_func = self.funcs.get(&name);
                if o_func.is_none() {
                    panic!("[ERROR] Function {} is undefined", name);
                }
                let (ret_type, id) = o_func.unwrap();
                if name == "str".to_string()
                    || name == "bool".to_string()
                    || name == "int".to_string()
                    || name == "float".to_string()
                {
                    self.inject_type_param(&last_type, false, _module, builder, &mut param_values);
                }
                if name == "print".to_string() || name == "println".to_string() {
                    self.inject_type_param(&last_type, true, _module, builder, &mut param_values);
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
            Ast::ArrRef(a, indices) => {
                let name = *a.clone();
                let (arr_var, mut arr_type) = scope.as_ref().borrow().get(name);
                let (_, arr_read_global) = self.funcs.get("toy_read_from_arr").unwrap();
                let arr_read = _module.declare_func_in_func(*arr_read_global, &mut builder.func);

                let mut current_ptr = builder.use_var(arr_var);

                for (dim, idx_expr) in indices.iter().enumerate() {
                    let (idx_val, _) = self.compile_expr(idx_expr, _module, builder, scope);

                    let call_params = [current_ptr, idx_val].to_vec();
                    let call_inst = builder.ins().call(arr_read, call_params.as_slice());
                    current_ptr = builder.inst_results(call_inst)[0];

                    arr_type = match arr_type {
                        TypeTok::IntArr(n) if n > 1 => TypeTok::IntArr(n - 1),
                        TypeTok::BoolArr(n) if n > 1 => TypeTok::BoolArr(n - 1),
                        TypeTok::StrArr(n) if n > 1 => TypeTok::StrArr(n - 1),
                        TypeTok::FloatArr(n) if n > 1 => TypeTok::FloatArr(n - 1),
                        TypeTok::AnyArr(n) if n > 1 => TypeTok::AnyArr(n - 1),
                        TypeTok::IntArr(1) => TypeTok::Int,
                        TypeTok::BoolArr(1) => TypeTok::Bool,
                        TypeTok::StrArr(1) => TypeTok::Str,
                        TypeTok::FloatArr(1) => TypeTok::Float,
                        TypeTok::AnyArr(1) => TypeTok::Any,
                        _ => panic!(
                            "[ERROR] Type mismatch while indexing array {:?} at dimension {}",
                            arr_type, dim + 1
                        ),
                    };
                }

                (current_ptr, arr_type)
            }

            Ast::ArrLit(t, val) => (self.compile_arr_lit(val, _module, builder, scope),t.clone()),
            Ast::IntLit(n) => (builder.ins().iconst(types::I64, *n), TypeTok::Int),
            Ast::FloatLit(f) => {
                let float = *f;
                //I have this
                (
                    builder.ins().iconst(types::I64, float.to_bits() as i64),
                    TypeTok::Float,
                )
            }
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
                if (l_type_str == "Float" && r_type_str == "Float")
                    || (l_type_str == "Float" && r_type_str == "Int")
                    || (l_type_str == "Int" && r_type_str == "Float")
                {
                    let lf = if l_type_str == "Int" {
                        let int_to_float = self
                            .funcs
                            .get("toy_int_to_float")
                            .expect("int_to_float not found");
                        let func_ref = _module.declare_func_in_func(int_to_float.1, builder.func);
                        let call_inst = builder.ins().call(func_ref, &[l]);
                        builder.inst_results(call_inst)[0]
                    } else {
                        let float_bits_to_double = self
                            .funcs
                            .get("toy_float_bits_to_double")
                            .expect("float_bits_to_double not found");
                        let func_ref =
                            _module.declare_func_in_func(float_bits_to_double.1, builder.func);
                        let call_inst = builder.ins().call(func_ref, &[l]);
                        builder.inst_results(call_inst)[0]
                    };

                    let rf = if r_type_str == "Int" {
                        let int_to_float = self
                            .funcs
                            .get("toy_int_to_float")
                            .expect("int_to_float not found");
                        let func_ref = _module.declare_func_in_func(int_to_float.1, builder.func);
                        let call_inst = builder.ins().call(func_ref, &[r]);
                        builder.inst_results(call_inst)[0]
                    } else {
                        let float_bits_to_double = self
                            .funcs
                            .get("toy_float_bits_to_double")
                            .expect("float_bits_to_double not found");
                        let func_ref =
                            _module.declare_func_in_func(float_bits_to_double.1, builder.func);
                        let call_inst = builder.ins().call(func_ref, &[r]);
                        builder.inst_results(call_inst)[0]
                    };

                    let result_f64 = match op {
                        InfixOp::Plus => builder.ins().fadd(lf, rf),
                        InfixOp::Minus => builder.ins().fsub(lf, rf),
                        InfixOp::Multiply => builder.ins().fmul(lf, rf),
                        InfixOp::Divide => builder.ins().fdiv(lf, rf),
                        InfixOp::Modulo => {
                            let div = builder.ins().fdiv(lf, rf);
                            let floored = builder.ins().floor(div);
                            let prod = builder.ins().fmul(rf, floored);
                            builder.ins().fsub(lf, prod)
                        }
                        InfixOp::LessThan => {
                            let cmp = builder.ins().fcmp(FloatCC::LessThan, lf, rf);
                            return (builder.ins().uextend(types::I64, cmp), TypeTok::Bool);
                        }
                        InfixOp::LessThanEqt => {
                            let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, lf, rf);
                            return (builder.ins().uextend(types::I64, cmp), TypeTok::Bool);
                        }
                        InfixOp::GreaterThan => {
                            let cmp = builder.ins().fcmp(FloatCC::GreaterThan, lf, rf);
                            return (builder.ins().uextend(types::I64, cmp), TypeTok::Bool);
                        }
                        InfixOp::GreaterThanEqt => {
                            let cmp = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lf, rf);
                            return (builder.ins().uextend(types::I64, cmp), TypeTok::Bool);
                        }
                        InfixOp::Equals => {
                            let cmp = builder.ins().fcmp(FloatCC::Equal, lf, rf);
                            return (builder.ins().uextend(types::I64, cmp), TypeTok::Bool);
                        }
                        InfixOp::NotEquals => {
                            let cmp = builder.ins().fcmp(FloatCC::NotEqual, lf, rf);
                            return (builder.ins().uextend(types::I64, cmp), TypeTok::Bool);
                        }
                        _ => panic!("[ERROR] Unsupported floating-point operation: {}", op),
                    };

                    // Convert result back to I64 bit representation
                    let double_to_bits = self
                        .funcs
                        .get("toy_double_to_float_bits")
                        .expect("double_to_float_bits not found");
                    let func_ref = _module.declare_func_in_func(double_to_bits.1, builder.func);
                    let call_inst = builder.ins().call(func_ref, &[result_f64]);
                    let result_bits = builder.inst_results(call_inst)[0];

                    return (result_bits, TypeTok::Float);
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
