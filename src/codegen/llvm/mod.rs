use std::collections::HashMap;

use inkwell::{AddressSpace, basic_block::BasicBlock, builder::Builder, context::Context, module::{Linkage, Module}, targets::{TargetMachineOptions, TargetTriple}, types::{BasicMetadataTypeEnum, BasicTypeEnum, FunctionType}, values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FloatValue, FunctionValue, ValueKind}};

use crate::{codegen::{Block, Function, SSAValue, TIR, TirType, tir::ir::BlockId}, errors::{ToyError, ToyErrorType}};
use inkwell::{
    OptimizationLevel,
    targets::{
        InitializationConfig, Target, FileType,
    },
};
use std::path::Path;

use std::env;
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
    block_id_to_block: HashMap<BlockId, BasicBlock<'a>>
}
impl<'a> LlvmGenerator<'a> {
    pub fn new(ctx: &'a Context, main_module: Module<'a>) -> LlvmGenerator<'a> {
        return LlvmGenerator {
            ctx,
            main_module,
            tir_to_val: HashMap::new(),
            func_map: HashMap::new(),
            block_id_to_block: HashMap::new()
        }
    }
    fn compile_instruction(&mut self, inst: TIR, builder: &Builder<'a>, curr_func_name: String) ->Result<(), ToyError> {
        let (llvm_ir, val) =  match inst {
            //this will cause bugs not accounting for bools
            TIR::IConst(id, val, ty) => (self.ctx.i64_type().const_int(val as u64, true).as_basic_value_enum(), SSAValue{val: id, ty: Some(ty)}), //this as u64 scares the shit out of me, LLVM should reinterpret the bits with twos complement but who the hell knows
            TIR::ItoF(id, ssa_ref, ty) => {
                let ins: FloatValue<'_> = builder.build_signed_int_to_float(
                    self.tir_to_val.get(&(curr_func_name.clone(), ssa_ref)).unwrap().into_int_value(),
                    self.ctx.f64_type(),
                    "sitofp" 
                     
                )?;
                (ins.into(), SSAValue{val: id, ty: Some(ty)})
            }
            //I am going to rush ahead to call_extern without dealing with any other function types for debuggability purposes
            TIR::CallExternFunction(id, name, params, _, ret_type) => {
                let hm_func = self.func_map.get(&*name).unwrap();
                let func_body = self.main_module.add_function(&*name, hm_func.to_owned(), Some(Linkage::External));
                let llvm_params: Vec<BasicMetadataValueEnum> = params.iter().map(|p| {self.tir_to_val.get(&((curr_func_name).clone(), p.clone())).unwrap()}.to_owned().into()).collect();
                let call_ins = builder.build_call(func_body, llvm_params.as_slice(), &*name)?;
                let ret = if ret_type !=TirType::Void { match call_ins.try_as_basic_value() {
                    ValueKind::Basic(v) => v,
                    _ => panic!("void"),
                }} else {
                    self.ctx.i64_type().const_int(0 as u64, true).into() //TIR-Gen will make sure this is never called
                };
                (ret, SSAValue{val: id, ty: Some(ret_type)})

            }
            TIR::Ret(id, v) => {
                //there might be a bug here with not adding the return val on the else branch to the tir_to_val
                if v.ty.is_none() {
                    builder.build_return(None)?;
                    (self.ctx.i64_type().const_int(0 as u64, true).into(), SSAValue{val: id, ty: None})
                } else {
                    let val = self.tir_to_val.get(&(curr_func_name.clone(), v.clone())).unwrap();
                    builder.build_return(Some(val))?;
                    (val.to_owned(), v)
                }
            }
            _ => todo!("Chase you have not implemented {:?} ins yet", inst)
        };
        self.tir_to_val.insert((curr_func_name, val), llvm_ir);
        return Ok(());
    }
    fn compile_tir_block(&mut self, tir_block: Block, builder: &Builder<'a>, func: FunctionValue<'a>, name: String) -> Result<BasicBlock<'a>, ToyError>{
        let llvm_block = self.ctx.append_basic_block(func, &name);
        builder.position_at_end(llvm_block);
        for ins in tir_block.ins {
            self.compile_instruction(ins, builder, name.clone())?;
        }

        return Ok(llvm_block);
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
        
        let llvm_func = self.main_module.add_function(&*func.name.clone(), fn_type, Some(if &*func.name == "user_main" {Linkage::External} else {Linkage::Internal}));
        //</Boiler plate to setup function>
        for (n, p) in func.params.iter().enumerate() {
            let p_val = llvm_func.get_nth_param(n as u32).unwrap();//is probably safe, maybe will cause bugs :D
            self.tir_to_val.insert((*func.name.clone(), p.clone()), p_val);
        }
        for b in func.body {
            let llvm_block = self.compile_tir_block(b.clone(), &builder, llvm_func, *func.name.clone())?;//safety: Clone is the main one, only id is used after
            self.block_id_to_block.insert(b.id, llvm_block);
        }

        return Ok(());
    }
    fn _tir_to_llvm_type(&self, t: TirType) -> BasicTypeEnum<'a> {
        return match t {
            TirType::I64 => self.ctx.i64_type().into(),
            TirType::F64 => self.ctx.f64_type().into(),
            TirType::I1 => self.ctx.bool_type().into(),
            TirType::I8PTR => self.ctx.ptr_type(AddressSpace::default()).into(),
            _ => todo!("Chase you have not implemented this param type yet")
        };
    }
    fn declare_individual_function(&mut self, name: &str, types: Vec<TirType>, ret_type: TirType) {
        let mut compiled_types: Vec<BasicMetadataTypeEnum> = vec![];
        types.iter().for_each(|t| {
            compiled_types.push(self._tir_to_llvm_type(t.clone()).into())
        });

        let func: FunctionType = match ret_type {
            TirType::I64 => self.ctx.i64_type().fn_type(&compiled_types.as_slice(), false),
            TirType::F64 => self.ctx.f64_type().fn_type(&compiled_types.as_slice(), false),
            TirType::I1 => self.ctx.bool_type().fn_type(&compiled_types.as_slice(), false),
            TirType::I8PTR => self.ctx.ptr_type(AddressSpace::default()).fn_type(&compiled_types.as_slice(), false),
            TirType::Void => self.ctx.void_type().fn_type(&compiled_types.as_slice(), false),
            _ => todo!("Chase you have not implemented this return type yet")
        };
        self.func_map.insert(name.to_string(), func);

    }
    fn declare_builtin_functions(&mut self) -> Result<(), ToyError> {
        self.declare_individual_function(
            "toy_print",
            vec![
                TirType::I64,
                TirType::I64,
                TirType::I64
            ],
            TirType::Void
        );
        self.declare_individual_function(
            "toy_println",
            vec![TirType::I64,TirType::I64,TirType::I64],
            TirType::Void
        );
        self.declare_individual_function(
            "toy_malloc",
            vec![TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_concat",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_strequal",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_strlen",
            vec![TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_toy_type_to_str",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_type_to_bool",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_type_to_int",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_type_to_float",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_int_to_float",
            vec![TirType::I64],
            TirType::F64
        );
        self.declare_individual_function(
            "toy_float_bits_to_double",
            vec![TirType::I64],
            TirType::F64
        );
        self.declare_individual_function(
            "toy_double_to_float_bits",
            vec![TirType::F64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_malloc_arr",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_write_to_arr",
            vec![TirType::I64, TirType::I64, TirType::I64],
            TirType::Void
        );
        self.declare_individual_function(
            "toy_read_from_arr",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_arrlen",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        self.declare_individual_function(
            "toy_input",
            vec![TirType::I64, TirType::I64],
            TirType::I64
        );
        return Ok(())

    }
    fn generate_internal(&mut self, funcs: Vec<Function>) -> Result<(), ToyError> {
        self.declare_builtin_functions()?;
        for func in funcs {
            self.compile_tir_function(func)?;
        }
        return Ok(())
    }
    pub fn generate(&mut self, funcs: Vec<Function>, prgm_name: String) -> Result<(), ToyError> {
        self.generate_internal(funcs)?;

        //llvm shit
        let args: Vec<String> = env::args().collect();
        Target::initialize_x86(&InitializationConfig::default());
        let opt_level = if args.contains(&"--repl".to_string()) || args.contains(&"--no-op".to_string()) {
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
        let target_machine = target.create_target_machine_from_options(&triple, options).unwrap();
        self.main_module.set_triple(&triple);
        self.main_module.set_data_layout(
        &target_machine
            .get_target_data()
            .get_data_layout()
        );
        let obj_path = Path::new("temp/out.o");
        target_machine
        .write_to_file(
            &self.main_module,
            FileType::Object,
            obj_path
        )?;
        self.main_module.print_to_file(Path::new("temp/out.ll"))?;

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
                output_name.as_str()
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

        let rstatus = Command::new(lib_path.join("ld.lld")).args(args.clone()).status();
        let status = match rstatus {
            Ok(f) => f,
            Err(_) => return Err(ToyError::new(ToyErrorType::InternalLinkerFailure)),
        };
        if !status.success() {
            return Err(ToyError::new(ToyErrorType::InternalLinkerFailure));
        }
        //it should automatically run the program in the repl, it doesn't, I will fix i later I have more important things todo
        if args.clone().contains(&&"--repl") || args.clone().contains(&&"--run") {
            let mut prgm = Command::new(format!("{}{}", "./",output_name.as_str()));
            match prgm.spawn(){
                Ok(_) => {},
                Err(_) => return Err(ToyError::new(ToyErrorType::InternalLinkerFailure))//is his the right error
            }
        }
        return Ok(());
    }
}


#[cfg(test)]
mod tests;