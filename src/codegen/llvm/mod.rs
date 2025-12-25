use std::collections::HashMap;

use inkwell::{
    AddressSpace,
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    targets::{TargetMachineOptions, TargetTriple},
    types::{BasicMetadataTypeEnum, BasicTypeEnum, FunctionType},
    values::{
        BasicMetadataValueEnum, BasicValue, BasicValueEnum, FloatValue, FunctionValue, PhiValue,
        ValueKind,
    },
};
use inkwell::{FloatPredicate, IntPredicate};

use crate::{
    codegen::{
        Block, Function, SSAValue, TIR, TirType,
        tir::ir::{BlockId, BoolInfixOp, NumericInfixOp},
    },
    errors::{ToyError, ToyErrorType},
};
use inkwell::{
    OptimizationLevel,
    targets::{FileType, InitializationConfig, Target},
};
use std::path::Path;

use std::env;
use std::fs;
use std::process::Command;
pub static FILE_EXTENSION_EXE: &str = if cfg!(target_os = "windows") {
    ".exe"
} else {
    ""
};
pub struct LlvmGenerator<'a> {
    ctx: &'a Context,
    main_module: Module<'a>,
    tir_to_val: HashMap<(String, SSAValue), BasicValueEnum<'a>>,
    func_map: HashMap<String, FunctionType<'a>>,
    block_id_to_block: HashMap<BlockId, BasicBlock<'a>>,
    curr_func: Option<FunctionValue<'a>>,
    curr_tir_func: Option<Function>,
    phi_fixups: Vec<(PhiValue<'a>, String, BlockId, SSAValue)>,
}
impl<'a> LlvmGenerator<'a> {
    pub fn new(ctx: &'a Context, main_module: Module<'a>) -> LlvmGenerator<'a> {
        return LlvmGenerator {
            ctx,
            main_module,
            tir_to_val: HashMap::new(),
            func_map: HashMap::new(),
            block_id_to_block: HashMap::new(),
            curr_func: None,
            curr_tir_func: None,
            phi_fixups: vec![],
        };
    }
    fn get_ssa_val(&self, func_name: &str, ssa: SSAValue) -> BasicValueEnum<'a> {
        if let Some(v) = self.tir_to_val.get(&(func_name.to_string(), ssa.clone())) {
            return *v;
        }
        // Try alternative types for pointers/ints
        if let Some(ty) = &ssa.ty {
            let alt_ty = match ty {
                TirType::I64 => Some(TirType::I8PTR),
                TirType::I8PTR => Some(TirType::I64),
                _ => None,
            };
            if let Some(at) = alt_ty {
                let alt_ssa = SSAValue {
                    val: ssa.val,
                    ty: Some(at),
                };
                if let Some(v) = self.tir_to_val.get(&(func_name.to_string(), alt_ssa)) {
                    return *v;
                }
            }
        }
        panic!("Undefined SSA Value: {:?} in function {}", ssa, func_name);
    }
    fn compile_instruction(
        &mut self,
        inst: TIR,
        builder: &Builder<'a>,
        curr_func_name: String,
    ) -> Result<(), ToyError> {
        let res = match inst {
            //this will cause bugs not accounting for bools
            TIR::IConst(id, val, ty) => Some((
                self.ctx
                    .i64_type()
                    .const_int((val as i64) as u64, true)
                    .as_basic_value_enum(),
                SSAValue {
                    val: id,
                    ty: Some(ty),
                },
            )), //this as u64 scares the shit out of me, LLVM should reinterpret the bits with twos complement but who the hell knows
            TIR::FConst(id, val, ty) => Some((
                self.ctx
                    .f64_type()
                    .const_float(val as f64)
                    .as_basic_value_enum(),
                SSAValue {
                    val: id,
                    ty: Some(ty),
                },
            )),
            TIR::ItoF(id, ssa_ref, ty) => {
                let ins: FloatValue<'_> = builder.build_signed_int_to_float(
                    self.get_ssa_val(&curr_func_name, ssa_ref).into_int_value(),
                    self.ctx.f64_type(),
                    "sitofp",
                )?;
                Some((
                    ins.into(),
                    SSAValue {
                        val: id,
                        ty: Some(ty),
                    },
                ))
            }
            //I am going to rush ahead to call_extern without dealing with any other function types for debuggability purposes
            TIR::CallExternFunction(id, name, params, _, ret_type) => {
                let func_body = if let Some(f) = self.main_module.get_function(&name) {
                    f
                } else {
                    let hm_func = self.func_map.get(&*name).unwrap();
                    self.main_module.add_function(
                        &*name,
                        hm_func.to_owned(),
                        Some(Linkage::External),
                    )
                };
                let param_types = func_body.get_type().get_param_types();
                let llvm_params: Vec<BasicMetadataValueEnum> = params
                    .iter()
                    .zip(param_types.iter())
                    .map(|(p, expected_type)| {
                        let v = self.get_ssa_val(&curr_func_name, p.clone());

                        if expected_type.is_int_type() && v.is_float_value() {
                            builder
                                .build_bit_cast(
                                    v.into_float_value(),
                                    self.ctx.i64_type(),
                                    "double_to_i64_bitcast",
                                )
                                .unwrap()
                                .into()
                        } else {
                            match p.ty.clone().unwrap() {
                                TirType::I1 => builder
                                    .build_int_z_extend(
                                        v.into_int_value(),
                                        self.ctx.i64_type(),
                                        "bool_to_i64",
                                    )
                                    .unwrap()
                                    .into(),
                                _ => v.to_owned().into(),
                            }
                        }
                    })
                    .collect();

                let call_ins = builder.build_call(func_body, llvm_params.as_slice(), &*name)?;
                let ret = if ret_type != TirType::Void {
                    match call_ins.try_as_basic_value() {
                        ValueKind::Basic(v) => v,
                        _ => panic!("void"),
                    }
                } else {
                    self.ctx.i64_type().const_int(0 as u64, true).into() //TIR-Gen will make sure this is never called
                };
                Some((
                    ret,
                    SSAValue {
                        val: id,
                        ty: Some(ret_type),
                    },
                ))
            }
            TIR::CallLocalFunction(id, name, params, _, ret_type) => {
                let func_body = if let Some(f) = self.main_module.get_function(&name) {
                    f
                } else {
                    let mut compiled_types: Vec<BasicMetadataTypeEnum> = vec![];
                    for p in &params {
                        let t = p.ty.as_ref().expect("Param type should be known");
                        compiled_types.push(self._tir_to_llvm_type(t.clone()).into());
                    }

                    let fn_type = match ret_type {
                        TirType::I64 => self.ctx.i64_type().fn_type(&compiled_types, false),
                        TirType::F64 => self.ctx.f64_type().fn_type(&compiled_types, false),
                        TirType::I1 => self.ctx.bool_type().fn_type(&compiled_types, false),
                        TirType::I8PTR => self
                            .ctx
                            .ptr_type(AddressSpace::default())
                            .fn_type(&compiled_types, false),
                        TirType::Void => self.ctx.void_type().fn_type(&compiled_types, false),
                        _ => todo!("Chase you have not implemented this return type yet"),
                    };

                    self.main_module
                        .add_function(&*name, fn_type, Some(Linkage::External))
                };
                let param_types = func_body.get_type().get_param_types();
                let llvm_params: Vec<BasicMetadataValueEnum> = params
                    .iter()
                    .zip(param_types.iter())
                    .map(|(p, expected_type)| {
                        let v = self.get_ssa_val(&curr_func_name, p.clone());

                        if expected_type.is_int_type() && v.is_float_value() {
                            builder
                                .build_bit_cast(
                                    v.into_float_value(),
                                    self.ctx.i64_type(),
                                    "double_to_i64_bitcast",
                                )
                                .unwrap()
                                .into()
                        } else {
                            match p.ty.clone().unwrap() {
                                TirType::I1 => builder
                                    .build_int_z_extend(
                                        v.into_int_value(),
                                        self.ctx.i64_type(),
                                        "bool_to_i64",
                                    )
                                    .unwrap()
                                    .into(),
                                _ => v.to_owned().into(),
                            }
                        }
                    })
                    .collect();

                let call_ins = builder.build_call(func_body, llvm_params.as_slice(), &*name)?;
                let ret = if ret_type != TirType::Void {
                    match call_ins.try_as_basic_value() {
                        ValueKind::Basic(v) => v,
                        _ => panic!("void"),
                    }
                } else {
                    self.ctx.i64_type().const_int(0 as u64, true).into() //TIR-Gen will make sure this is never called
                };
                Some((
                    ret,
                    SSAValue {
                        val: id,
                        ty: Some(ret_type),
                    },
                ))
            }

            TIR::Ret(id, v) => {
                //there might be a bug here with not adding the return val on the else branch to the tir_to_val
                if v.ty.is_none() {
                    builder.build_return(None)?;
                    Some((
                        self.ctx.i64_type().const_int(0 as u64, true).into(),
                        SSAValue { val: id, ty: None },
                    ))
                } else {
                    let val = self.get_ssa_val(&curr_func_name, v.clone());
                    builder.build_return(Some(&val))?;
                    Some((val.to_owned(), v))
                }
            }
            TIR::NumericInfix(id, l, r, op) => {
                let lhs = self.get_ssa_val(&curr_func_name, l.clone());
                let rhs = self.get_ssa_val(&curr_func_name, r.clone());
                //tirgen wil guarantee that they are both either int or float
                if l.ty.unwrap() == TirType::F64 {
                    //float arithmetic
                    let val = match op {
                        NumericInfixOp::Plus => builder.build_float_add(
                            lhs.into_float_value(),
                            rhs.into_float_value(),
                            "sum",
                        )?,
                        NumericInfixOp::Minus => builder.build_float_sub(
                            lhs.into_float_value(),
                            rhs.into_float_value(),
                            "diff",
                        )?,
                        NumericInfixOp::Multiply => builder.build_float_mul(
                            lhs.into_float_value(),
                            rhs.into_float_value(),
                            "product",
                        )?,
                        NumericInfixOp::Divide => builder.build_float_div(
                            lhs.into_float_value(),
                            rhs.into_float_value(),
                            "quotient",
                        )?,
                        NumericInfixOp::Modulo => builder.build_float_rem(
                            lhs.into_float_value(),
                            rhs.into_float_value(),
                            "fmod_res",
                        )?,
                    };
                    self.tir_to_val.insert(
                        (
                            curr_func_name.clone(),
                            SSAValue {
                                val: id,
                                ty: Some(TirType::F64),
                            },
                        ),
                        val.into(),
                    );
                    Some((
                        val.into(),
                        SSAValue {
                            val: id,
                            ty: Some(TirType::F64),
                        },
                    ))
                } else {
                    let val = match op {
                        NumericInfixOp::Plus => builder.build_int_add(
                            lhs.into_int_value(),
                            rhs.into_int_value(),
                            "sum",
                        )?,
                        NumericInfixOp::Minus => builder.build_int_sub(
                            lhs.into_int_value(),
                            rhs.into_int_value(),
                            "diff",
                        )?,
                        NumericInfixOp::Multiply => builder.build_int_mul(
                            lhs.into_int_value(),
                            rhs.into_int_value(),
                            "product",
                        )?,
                        NumericInfixOp::Divide => builder.build_int_signed_div(
                            lhs.into_int_value(),
                            rhs.into_int_value(),
                            "quotient",
                        )?,
                        NumericInfixOp::Modulo => builder.build_int_signed_rem(
                            lhs.into_int_value(),
                            rhs.into_int_value(),
                            "imod_res",
                        )?,
                    };
                    self.tir_to_val.insert(
                        (
                            curr_func_name.clone(),
                            SSAValue {
                                val: id,
                                ty: Some(TirType::I64),
                            },
                        ),
                        val.into(),
                    );
                    Some((
                        val.into(),
                        SSAValue {
                            val: id,
                            ty: Some(TirType::I64),
                        },
                    ))
                }
            }
            TIR::BoolInfix(id, l, r, op) => {
                let lhs = self.get_ssa_val(&curr_func_name, l.clone());
                let rhs = self.get_ssa_val(&curr_func_name, r.clone());
                let res = match op {
                    BoolInfixOp::And => {
                        builder.build_and(lhs.into_int_value(), rhs.into_int_value(), "and_res")?
                    }
                    BoolInfixOp::Or => {
                        builder.build_or(lhs.into_int_value(), rhs.into_int_value(), "or_res")?
                    }
                    _ => {
                        if l.ty.unwrap() == TirType::F64 {
                            match op {
                                BoolInfixOp::Equals => builder.build_float_compare(
                                    FloatPredicate::OEQ,
                                    lhs.into_float_value(),
                                    rhs.into_float_value(),
                                    "float_eq_res",
                                )?,
                                BoolInfixOp::NotEquals => builder.build_float_compare(
                                    FloatPredicate::ONE,
                                    lhs.into_float_value(),
                                    rhs.into_float_value(),
                                    "float_neq_res",
                                )?,
                                BoolInfixOp::GreaterThanEqt => builder.build_float_compare(
                                    FloatPredicate::OGE,
                                    lhs.into_float_value(),
                                    rhs.into_float_value(),
                                    "float_greater_then_eqt_res",
                                )?,
                                BoolInfixOp::GreaterThan => builder.build_float_compare(
                                    FloatPredicate::OGT,
                                    lhs.into_float_value(),
                                    rhs.into_float_value(),
                                    "float_greater_then_res",
                                )?,
                                BoolInfixOp::LessThenEqt => builder.build_float_compare(
                                    FloatPredicate::OLE,
                                    lhs.into_float_value(),
                                    rhs.into_float_value(),
                                    "float_less_then_eqt_res",
                                )?,
                                BoolInfixOp::LessThan => builder.build_float_compare(
                                    FloatPredicate::OLT,
                                    lhs.into_float_value(),
                                    rhs.into_float_value(),
                                    "float_greater_then_res",
                                )?,
                                _ => unreachable!(),
                            }
                        } else {
                            match op {
                                BoolInfixOp::Equals => builder.build_int_compare(
                                    IntPredicate::EQ,
                                    lhs.into_int_value(),
                                    rhs.into_int_value(),
                                    "int_eq_res",
                                )?,
                                BoolInfixOp::NotEquals => builder.build_int_compare(
                                    IntPredicate::NE,
                                    lhs.into_int_value(),
                                    rhs.into_int_value(),
                                    "int_neq_res",
                                )?,
                                BoolInfixOp::GreaterThanEqt => builder.build_int_compare(
                                    IntPredicate::SGE,
                                    lhs.into_int_value(),
                                    rhs.into_int_value(),
                                    "int_greater_then_eqt_res",
                                )?,
                                BoolInfixOp::GreaterThan => builder.build_int_compare(
                                    IntPredicate::SGT,
                                    lhs.into_int_value(),
                                    rhs.into_int_value(),
                                    "int_greater_then_res",
                                )?,
                                BoolInfixOp::LessThenEqt => builder.build_int_compare(
                                    IntPredicate::SLE,
                                    lhs.into_int_value(),
                                    rhs.into_int_value(),
                                    "int_less_then_eqt_res",
                                )?,
                                BoolInfixOp::LessThan => builder.build_int_compare(
                                    IntPredicate::SLT,
                                    lhs.into_int_value(),
                                    rhs.into_int_value(),
                                    "int_greater_then_res",
                                )?,
                                _ => unreachable!(),
                            }
                        }
                    }
                };
                self.tir_to_val.insert(
                    (
                        curr_func_name.clone(),
                        SSAValue {
                            val: id,
                            ty: Some(TirType::I1),
                        },
                    ),
                    res.into(),
                );
                Some((
                    res.into(),
                    SSAValue {
                        val: id,
                        ty: Some(TirType::I1),
                    },
                ))
            }
            TIR::Not(id, v) => {
                let val = self
                    .get_ssa_val(&curr_func_name, v.clone())
                    .into_int_value();

                let one = self.ctx.bool_type().const_int(1, false);

                let res = builder.build_xor(val, one, "not_res")?;

                Some((
                    res.into(),
                    SSAValue {
                        val: id,
                        ty: Some(TirType::I1),
                    },
                ))
            }
            TIR::JumpCond(_, cond, if_true, if_false) => {
                let compiled_cond = self.get_ssa_val(&curr_func_name, cond).into_int_value();
                let true_block = self.block_id_to_block.get(&if_true).unwrap();
                let false_block = self.block_id_to_block.get(&if_false).unwrap();
                builder.build_conditional_branch(compiled_cond, *true_block, *false_block)?;
                None
            }
            TIR::JumpBlockUnCond(_, block_id) => {
                let block = self.block_id_to_block.get(&block_id).unwrap();
                builder.build_unconditional_branch(*block)?;
                None
            }
            TIR::Phi(id, block_ids, vals) => {
                let first_val_ssa = &vals[0];
                let mut ty = None;
                for val in &vals {
                    if let Some(v) = self.tir_to_val.get(&(curr_func_name.clone(), val.clone())) {
                        ty = Some(v.get_type());
                        break;
                    }
                }

                if ty.is_none() {
                    if let Some(t) = &first_val_ssa.ty {
                        ty = Some(self._tir_to_llvm_type(t.clone()));
                    }
                }

                let ty = ty.expect("Could not determine type for Phi node");

                let phi = builder.build_phi(ty, "phi")?;

                for (block_id, ssa_val) in block_ids.iter().zip(vals.iter()) {
                    let block = self.block_id_to_block.get(block_id).unwrap();
                    if let Some(val) = self
                        .tir_to_val
                        .get(&(curr_func_name.clone(), ssa_val.clone()))
                    {
                        phi.add_incoming(&[(&*val, *block)]);
                    } else {
                        // Try alternative type lookup for phi incoming values
                        let mut found = false;
                        if let Some(ty) = &ssa_val.ty {
                            let alt_ty = match ty {
                                TirType::I64 => Some(TirType::I8PTR),
                                TirType::I8PTR => Some(TirType::I64),
                                _ => None,
                            };
                            if let Some(at) = alt_ty {
                                let alt_ssa = SSAValue {
                                    val: ssa_val.val,
                                    ty: Some(at),
                                };
                                if let Some(val) =
                                    self.tir_to_val.get(&(curr_func_name.clone(), alt_ssa))
                                {
                                    phi.add_incoming(&[(&*val, *block)]);
                                    found = true;
                                }
                            }
                        }

                        if !found {
                            self.phi_fixups.push((
                                phi,
                                curr_func_name.clone(),
                                *block_id,
                                ssa_val.clone(),
                            ));
                        }
                    }
                }

                Some((
                    phi.as_basic_value(),
                    SSAValue {
                        val: id,
                        ty: first_val_ssa.ty.clone(),
                    },
                ))
            }
            TIR::GlobalString(id, va) => {
                let str_val =
                    builder.build_global_string_ptr(&va, format!("global_str_{}", id).as_str())?;
                let str_ptr = str_val.as_pointer_value();
                let ptr_i64 =
                    builder.build_ptr_to_int(str_ptr, self.ctx.i64_type(), "str_ptr_to_i64")?;
                self.tir_to_val.insert(
                    (
                        curr_func_name.clone(),
                        SSAValue {
                            val: id,
                            ty: Some(TirType::I64),
                        },
                    ),
                    ptr_i64.into(),
                );
                Some((
                    ptr_i64.into(),
                    SSAValue {
                        val: id,
                        ty: Some(TirType::I8PTR),
                    },
                ))
            }
            _ => todo!("Chase you have not implemented {:?} ins yet", inst),
        };
        if let Some((llvm_ir, val)) = res {
            self.tir_to_val.insert((curr_func_name, val), llvm_ir);
        }
        return Ok(());
    }
    fn compile_tir_block(
        &mut self,
        tir_block: Block,
        builder: &Builder<'a>,
        llvm_block: BasicBlock<'a>,
        name: String,
    ) -> Result<(), ToyError> {
        builder.position_at_end(llvm_block);
        for ins in tir_block.ins {
            self.compile_instruction(ins, builder, name.clone())?;
        }

        return Ok(());
    }
    fn compile_tir_function(&mut self, func: Function) -> Result<(), ToyError> {
        self.curr_tir_func = Some(func.clone());
        //<Boiler plate to setup function>
        let builder = self.ctx.create_builder();
        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![];
        for p in func.params.clone() {
            llvm_params.push(match p.ty {
                Some(t) => match t {
                    TirType::I64 => self.ctx.i64_type().into(),
                    TirType::F64 => self.ctx.f64_type().into(),
                    TirType::I1 => self.ctx.bool_type().into(),
                    TirType::I8PTR => self.ctx.ptr_type(AddressSpace::default()).into(),
                    _ => todo!("Chase you have not implemented this param type yet"),
                },
                None => unreachable!(), //SAFETY: Guaranteed by ast-gen
            })
        }
        let fn_type = match func.ret_type {
            TirType::I64 => self.ctx.i64_type().fn_type(llvm_params.as_slice(), false),
            TirType::F64 => self.ctx.f64_type().fn_type(llvm_params.as_slice(), false),
            TirType::I1 => self.ctx.bool_type().fn_type(llvm_params.as_slice(), false),
            TirType::I8PTR => self
                .ctx
                .ptr_type(AddressSpace::default())
                .fn_type(llvm_params.as_slice(), false),
            _ => todo!("Chase you have not implemented this return type yet"),
        };

        let llvm_func = if let Some(f) = self.main_module.get_function(&func.name) {
            if &*func.name != "user_main" {
                f.set_linkage(Linkage::Internal);
            }
            f
        } else {
            self.main_module.add_function(
                &*func.name.clone(),
                fn_type,
                Some(if &*func.name == "user_main" {
                    Linkage::External
                } else {
                    Linkage::Internal
                }),
            )
        };
        self.curr_func = Some(llvm_func);
        //</Boiler plate to setup function>
        for (n, p) in func.params.iter().enumerate() {
            let p_val = llvm_func.get_nth_param(n as u32).unwrap(); //is probably safe, maybe will cause bugs :D
            self.tir_to_val.insert(
                (
                    *self.curr_tir_func.as_ref().unwrap().name.clone(),
                    p.clone(),
                ),
                p_val,
            );
        }

        for b in &func.body {
            let llvm_block = self
                .ctx
                .append_basic_block(llvm_func, &format!("block_{}", b.id));
            self.block_id_to_block.insert(b.id, llvm_block);
        }

        for b in &func.body {
            let llvm_block = self.block_id_to_block.get(&b.id).unwrap();
            self.compile_tir_block(b.clone(), &builder, *llvm_block, *func.name.clone())?;
        }

        let fixups: Vec<_> = self.phi_fixups.drain(..).collect();
        for (phi, func_name, block_id, ssa_val) in fixups {
            let block = self.block_id_to_block.get(&block_id).unwrap();
            let val = self.get_ssa_val(&func_name, ssa_val);
            phi.add_incoming(&[(&val, *block)]);
        }

        return Ok(());
    }
    fn _tir_to_llvm_type(&self, t: TirType) -> BasicTypeEnum<'a> {
        return match t {
            TirType::I64 => self.ctx.i64_type().into(),
            TirType::F64 => self.ctx.f64_type().into(),
            TirType::I1 => self.ctx.bool_type().into(),
            TirType::I8PTR => self.ctx.ptr_type(AddressSpace::default()).into(),
            _ => todo!("Chase you have not implemented this param type yet"),
        };
    }
    fn declare_individual_function(&mut self, name: &str, types: Vec<TirType>, ret_type: TirType) {
        let mut compiled_types: Vec<BasicMetadataTypeEnum> = vec![];
        types
            .iter()
            .for_each(|t| compiled_types.push(self._tir_to_llvm_type(t.clone()).into()));

        let func: FunctionType = match ret_type {
            TirType::I64 => self
                .ctx
                .i64_type()
                .fn_type(&compiled_types.as_slice(), false),
            TirType::F64 => self
                .ctx
                .f64_type()
                .fn_type(&compiled_types.as_slice(), false),
            TirType::I1 => self
                .ctx
                .bool_type()
                .fn_type(&compiled_types.as_slice(), false),
            TirType::I8PTR => self
                .ctx
                .ptr_type(AddressSpace::default())
                .fn_type(&compiled_types.as_slice(), false),
            TirType::Void => self
                .ctx
                .void_type()
                .fn_type(&compiled_types.as_slice(), false),
            _ => todo!("Chase you have not implemented this return type yet"),
        };
        self.func_map.insert(name.to_string(), func);
    }
    fn declare_builtin_functions(&mut self) -> Result<(), ToyError> {
        self.declare_individual_function(
            "toy_print",
            vec![TirType::I64, TirType::I64, TirType::I64],
            TirType::Void,
        );
        self.declare_individual_function(
            "toy_println",
            vec![TirType::I64, TirType::I64, TirType::I64],
            TirType::Void,
        );
        self.declare_individual_function("toy_malloc", vec![TirType::I64], TirType::I64);
        self.declare_individual_function(
            "toy_concat",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_strequal",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function("toy_strlen", vec![TirType::I64], TirType::I64);
        self.declare_individual_function(
            "toy_toy_type_to_str",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_type_to_bool",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_type_to_int",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_type_to_float",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function("toy_int_to_float", vec![TirType::I64], TirType::F64);
        self.declare_individual_function(
            "toy_float_bits_to_double",
            vec![TirType::I64],
            TirType::F64,
        );
        self.declare_individual_function(
            "toy_double_to_float_bits",
            vec![TirType::F64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_malloc_arr",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_write_to_arr",
            vec![TirType::I64, TirType::I64, TirType::I64, TirType::I64],
            TirType::Void,
        );
        self.declare_individual_function(
            "toy_read_from_arr",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_arrlen",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_input",
            vec![TirType::I64, TirType::I64],
            TirType::I64,
        );
        self.declare_individual_function(
            "toy_free",
            vec![TirType::I64], //?
            TirType::Void,
        );
        return Ok(());
    }
    fn generate_internal(&mut self, funcs: Vec<Function>) -> Result<(), ToyError> {
        self.declare_builtin_functions()?;
        for func in funcs {
            self.compile_tir_function(func)?;
        }
        return Ok(());
    }
    pub fn generate(&mut self, funcs: Vec<Function>, prgm_name: String) -> Result<(), ToyError> {
        self.generate_internal(funcs)?;

        //llvm shit
        let args: Vec<String> = env::args().collect();
        Target::initialize_x86(&InitializationConfig::default());
        let opt_level =
            if args.contains(&"--repl".to_string()) || args.contains(&"--no-op".to_string()) {
                OptimizationLevel::None
            } else {
                OptimizationLevel::Aggressive
            };
        let triple = TargetTriple::create(if cfg!(target_os = "windows") {
            "x86_64-pc-windows-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        });
        let options = TargetMachineOptions::new()
            .set_abi("gnu")
            //.set_cpu("x64")
            .set_level(opt_level);
        let target = Target::from_triple(&triple).unwrap();
        let target_machine = target
            .create_target_machine_from_options(&triple, options)
            .unwrap();
        self.main_module.set_triple(&triple);
        self.main_module
            .set_data_layout(&target_machine.get_target_data().get_data_layout());

        let obj_file = format!("{}.o", prgm_name);
        let obj_path = Path::new(&obj_file);
        target_machine.write_to_file(&self.main_module, FileType::Object, obj_path)?;
        let ll_file = format!("{}.ll", prgm_name);
        self.main_module.print_to_file(Path::new(&ll_file))?;

        //linker
        let target = env!("TARGET").replace("\"", "");
        let lib_str = format!("lib/{}/", target);
        let lib_path = Path::new(&lib_str);
        let crt2_path = lib_path.join("crt2.o");
        let crtbegin_path = lib_path.join("crtbegin.o");
        let crt1_path = lib_path.join("crt1.o");
        let crti_path = lib_path.join("crti.o");
        let lbruntime_path = lib_path.join("libruntime.a");
        let crtn_path = lib_path.join("crtn.o");
        let libc_path = lib_path.join("libc.so.6");
        let libm_path = lib_path.join("libm.so.6");
        let output_name = format!("{}{}", prgm_name, FILE_EXTENSION_EXE);
        let args: Vec<&str> = if env::consts::OS == "windows" {
            vec![
                "-m",
                "i386pep",
                crt2_path.to_str().unwrap(),
                crtbegin_path.to_str().unwrap(),
                obj_path.to_str().unwrap(),
                "-L",
                lib_path.to_str().unwrap(),
                "-lruntime",
                "-lmingw32",
                "-lmingwex",
                "-lmsvcrt",
                "-lkernel32",
                "-luser32",
                "-lshell32",
                "-lgcc",
                "-o",
                output_name.as_str(),
            ]
        } else {
            vec![
                "-m",
                "elf_x86_64",
                crt1_path.to_str().unwrap(),
                crti_path.to_str().unwrap(),
                obj_path.to_str().unwrap(),
                lbruntime_path.to_str().unwrap(),
                crtn_path.to_str().unwrap(),
                libc_path.to_str().unwrap(),
                libm_path.to_str().unwrap(),
                "-dynamic-linker",
                "/lib64/ld-linux-x86-64.so.2",
                "-o",
                output_name.as_str(),
            ]
        };
        let rstatus = Command::new(lib_path.join("ld.lld"))
            .args(args.clone())
            .status();
        let status = match rstatus {
            Ok(f) => f,
            Err(_) => return Err(ToyError::new(ToyErrorType::InternalLinkerFailure)),
        };
        if !status.success() {
            return Err(ToyError::new(ToyErrorType::InternalLinkerFailure));
        }
        let p_args: Vec<String> = env::args().collect();
        if p_args.clone().contains(&"--repl".to_owned())
            || p_args.clone().contains(&"--run".to_owned())
        {
            let mut prgm = Command::new(format!("{}{}", "./", output_name.as_str()));
            let _ = prgm.spawn().unwrap().wait().unwrap();

            fs::remove_file(output_name.as_str()).unwrap();
            if !p_args.contains(&"--save-temps".to_string()) {
                fs::remove_file(format!("{}.o", prgm_name)).unwrap();
                fs::remove_file(format!("{}.ll", prgm_name)).unwrap();
            }
        }
        return Ok(());
    }
}

#[cfg(test)]
mod tests;
