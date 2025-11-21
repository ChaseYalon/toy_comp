use super::super::Compiler;
use super::Scope;
use crate::parser::ast::{Ast, InfixOp};
use crate::token::TypeTok;
use std::cell::RefCell;
use std::rc::Rc;

use cranelift::prelude::*;
use cranelift_module::Module;
use crate::errors::{ToyError, ToyErrorType};
impl Compiler {
    fn compile_string_infix<M: Module>(
        &mut self,
        left: &Value,
        right: &Value,
        op: &InfixOp,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<(Value, TypeTok), ToyError> {
        return match op {
            InfixOp::Plus => {
                let toy_concat = self.funcs.get("concat").ok_or_else(||{return ToyError::new(ToyErrorType::InternalFunctionUndefined)})?;

                let func_ref = module.declare_func_in_func(toy_concat.1, builder.func);
                let call_inst = builder.ins().call(func_ref, &[left.clone(), right.clone()]);
                let results = builder.inst_results(call_inst);
                let heap_ptr = results[0];
                Ok((heap_ptr, TypeTok::Str))
            }
            InfixOp::Equals => {
                let toy_strequal = self.funcs.get("strequal").ok_or_else(||return ToyError::new(ToyErrorType::InternalFunctionUndefined))?;
                let func_ref = module.declare_func_in_func(toy_strequal.1, builder.func);
                let call_inst = builder.ins().call(func_ref, &[left.clone(), right.clone()]);
                let results = builder.inst_results(call_inst);
                let heap_ptr = results[0];
                Ok((heap_ptr, TypeTok::Bool))
            }
            InfixOp::NotEquals => {
                let toy_strequal = self.funcs.get("strequal").ok_or_else(||return ToyError::new(ToyErrorType::InternalFunctionUndefined))?;
                let func_ref = module.declare_func_in_func(toy_strequal.1, builder.func);
                let call_inst = builder.ins().call(func_ref, &[left.clone(), right.clone()]);
                let results = builder.inst_results(call_inst);
                let heap_ptr = results[0];

                //inverts result of eq
                let one = builder.ins().iconst(types::I64, 1);
                let flipped = builder.ins().bxor(heap_ptr, one);

                Ok((flipped, TypeTok::Bool))
            }
            _ => Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType))
        };
    }
    fn compile_int_expression<M: Module>(
        &mut self,
        l: Value,
        r: Value,
        op: &InfixOp,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<(Value, TypeTok), ToyError> {
        let to_ret = match op {
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
            _ => return Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType)),
        };
        return Ok(to_ret)
    }
    fn compile_partially_or_fully_float_expression<M: Module>(
        &mut self,
        left: Value,
        right: Value,
        l_type_str: String,
        r_type_str: String,
        op: &InfixOp,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<(Value, TypeTok), ToyError> {
        let lf = if l_type_str == "Int" {
            let int_to_float = self
                .funcs
                .get("toy_int_to_float")
                .ok_or_else(||return ToyError::new(ToyErrorType::InternalFunctionUndefined))?;
            let func_ref = module.declare_func_in_func(int_to_float.1, builder.func);
            let call_inst = builder.ins().call(func_ref, &[left]);
            builder.inst_results(call_inst)[0]
        } else {
            let float_bits_to_double = self
                .funcs
                .get("toy_float_bits_to_double")
                .ok_or_else(||return ToyError::new(ToyErrorType::InternalFunctionUndefined))?;
            let func_ref = module.declare_func_in_func(float_bits_to_double.1, builder.func);
            let call_inst = builder.ins().call(func_ref, &[left]);
            builder.inst_results(call_inst)[0]
        };

        let rf = if r_type_str == "Int" {
            let int_to_float = self
                .funcs
                .get("toy_int_to_float")
                .ok_or_else(||return ToyError::new(ToyErrorType::InternalFunctionUndefined))?;
            let func_ref = module.declare_func_in_func(int_to_float.1, builder.func);
            let call_inst = builder.ins().call(func_ref, &[right]);
            builder.inst_results(call_inst)[0]
        } else {
            let float_bits_to_double = self
                .funcs
                .get("toy_float_bits_to_double")
                .ok_or_else(||return ToyError::new(ToyErrorType::InternalFunctionUndefined))?;
            let func_ref = module.declare_func_in_func(float_bits_to_double.1, builder.func);
            let call_inst = builder.ins().call(func_ref, &[right]);
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
                return Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool));
            }
            InfixOp::LessThanEqt => {
                let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, lf, rf);
                return Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool));
            }
            InfixOp::GreaterThan => {
                let cmp = builder.ins().fcmp(FloatCC::GreaterThan, lf, rf);
                return Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool));
            }
            InfixOp::GreaterThanEqt => {
                let cmp = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lf, rf);
                return Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool));
            }
            InfixOp::Equals => {
                let cmp = builder.ins().fcmp(FloatCC::Equal, lf, rf);
                return Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool));
            }
            InfixOp::NotEquals => {
                let cmp = builder.ins().fcmp(FloatCC::NotEqual, lf, rf);
                return Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool));
            }
            _ => return Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType)),
        };

        // Convert result back to I64 bit representation
        let double_to_bits = self
            .funcs
            .get("toy_double_to_float_bits")
            .ok_or_else(||return ToyError::new(ToyErrorType::InternalFunctionUndefined))?;
        let func_ref = module.declare_func_in_func(double_to_bits.1, builder.func);
        let call_inst = builder.ins().call(func_ref, &[result_f64]);
        let result_bits = builder.inst_results(call_inst)[0];

        return Ok((result_bits, TypeTok::Float));
    }
    pub fn compile_infix_expression<M: Module>(
        &mut self,
        left: &Ast,
        right: &Ast,
        op: &InfixOp,
        module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Result<(Value, TypeTok), ToyError> {
        let (l, l_t) = self.compile_expr(left, module, builder, scope)?;
        let (r, r_t) = self.compile_expr(right, module, builder, scope)?;
        let l_type_str = l_t.type_str();
        let r_type_str = r_t.type_str();

        if l_type_str == "Int" && r_type_str == "Int" {
            return self.compile_int_expression(l, r, op, module, builder);
        }
        if l_type_str == "Bool" && r_type_str == "Bool" {
            return match op {
                InfixOp::Equals => {
                    let cmp = builder.ins().icmp(IntCC::Equal, l, r);
                    Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool))
                }
                InfixOp::NotEquals => {
                    let cmp = builder.ins().icmp(IntCC::NotEqual, l, r);
                    Ok((builder.ins().uextend(types::I64, cmp), TypeTok::Bool))
                }
                InfixOp::And => Ok((builder.ins().band(l, r), TypeTok::Bool)),
                InfixOp::Or => Ok((builder.ins().bor(l, r), TypeTok::Bool)),
                _ => Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType)),
            };
        }
        if l_type_str == "Str" && r_type_str == "Str" {
            return self.compile_string_infix(&l, &r, op, module, builder);
        }
        if (l_type_str == "Float" && r_type_str == "Float")
            || (l_type_str == "Float" && r_type_str == "Int")
            || (l_type_str == "Int" && r_type_str == "Float")
        {
            return self.compile_partially_or_fully_float_expression(
                l, r, l_type_str, r_type_str, op, module, builder,
            );
        }
        return Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType))
    }
}