use std::collections::HashMap;

use inkwell::{AddressSpace, basic_block::BasicBlock, context::Context, module::{Linkage, Module}, types::{BasicMetadataTypeEnum, BasicTypeEnum}, values::BasicValueEnum};

use crate::{codegen::{Block, Function, SSAValue, TIR, TirType}, errors::ToyError};



pub struct LlvmGenerator<'a> {
    ctx: &'a Context, 
    main_module: Module<'a>,
    tir_to_val: HashMap<(String, SSAValue), BasicValueEnum<'a>>
}
impl<'a> LlvmGenerator<'a> {
    pub fn new(ctx: &'a Context, main_module: Module<'a>) -> LlvmGenerator<'a> {
        return LlvmGenerator {
            ctx,
            main_module,
            tir_to_val: HashMap::new()
        }
    }
    fn compile_instruction(&mut self, inst: TIR) ->Result<(), ToyError> {
        return Ok(());
    }
    fn compile_tir_block(&mut self, tir_block: Block, m: &Module, ctx: &Context) -> Result<BasicBlock, ToyError>{
        for ins in tir_block.ins {

        }

        return Ok(());
    }
    fn compile_tir_function(&mut self, func: Function) -> Result<(), ToyError> {
        //<Boiler plate to setup function>
        let builder = self.ctx.create_builder();
        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![];
        for p in func.params.clone() {
            llvm_params.push(
                match p.ty {
                    Some(t) => match t {
                        TirType::I64 => self.ctx.i64_type().into(),
                        TirType::F64 => self.ctx.f64_type().into(),
                        TirType::I1 => self.ctx.bool_type().into(),
                        TirType::I8PTR => self.ctx.ptr_type(AddressSpace::default()).into(),
                        _ => todo!("Chase you have not implemented this param type yet")
                    },
                    None => unreachable!()//SAFETY: Guaranteed by ast-gen   
                }
            )
        }
        let fn_type = match func.ret_type {
            TirType::I64 => self.ctx.i64_type().fn_type(llvm_params.as_slice(), false),
            TirType::F64 => self.ctx.f64_type().fn_type(llvm_params.as_slice(), false),
            TirType::I1 => self.ctx.bool_type().fn_type(llvm_params.as_slice(), false),
            TirType::I8PTR => self.ctx.ptr_type(AddressSpace::default()).fn_type(llvm_params.as_slice(), false),
            _ => todo!("Chase you have not implemented this return type yet")
        };
        
        let llvm_func = self.main_module.add_function(&*func.name.clone(), fn_type, Some(if &*func.name == "user_main" {Linkage::AvailableExternally} else {Linkage::Internal}));
        //</Boiler plate to setup function>
        for (n, p) in func.params.iter().enumerate() {
            let p_val = llvm_func.get_nth_param(n as u32).unwrap();//is probably safe, maybe will cause bugs :D
            self.tir_to_val.insert((*func.name.clone(), p.clone()), p_val);
        }

        return Ok(())

    }
    pub fn generate(&mut self, funcs: Vec<Function>) -> Result<(), ToyError> {
        for func in funcs {
            self.compile_tir_function(func)?;
        }
        return Ok(())
    }
}