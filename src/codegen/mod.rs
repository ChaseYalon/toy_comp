mod ctla;
mod linker;
mod llvm;
mod tir;
use crate::codegen::linker::FILE_EXTENSION_EXE;
use crate::codegen::linker::Linker;
use crate::codegen::llvm::LlvmGenerator;
use crate::errors::ToyErrorType;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::Ast;
use crate::{codegen::ctla::CTLA, errors::ToyError};
use inkwell::context::Context;
use inkwell::module::Module;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tir::AstToIrConverter;
pub use tir::ir::{Block, Function, SSAValue, TIR, TirType};

pub struct Generator<'a> {
    converter: AstToIrConverter,
    analyzer: CTLA,
    generator: LlvmGenerator<'a>,
    linker: Linker,
}

impl<'a> Generator<'a> {
    pub fn new(ctx: &'a Context, main_module: Module<'a>) -> Generator<'a> {
        Generator {
            converter: AstToIrConverter::new(),
            analyzer: CTLA::new(),
            generator: LlvmGenerator::new(ctx, main_module), //I hate that that is nesscary
            linker: Linker::new(),
        }
    }

    pub fn compile_to_object(
        &mut self,
        ast: Vec<Ast>,
        name: String,
        is_main: bool,
    ) -> Result<(), ToyError> {
        let _ = self.converter.convert(ast, is_main)?;
        let ir = self.analyzer.analyze(self.converter.builder.clone())?;
        self.generator.generate(ir, name)?;
        Ok(())
    }

    fn compile_dependency(&self, module_name: &str) -> Result<(), ToyError> {
        // Check if the object file already exists
        let obj_path_str = format!("{}.o", module_name);
        let obj_path = Path::new(&obj_path_str);
        if obj_path.exists() {
            return Ok(());
        }

        // Resolve path: replace dots with slashes
        let path_str = format!("{}.toy", module_name.replace(".", "/"));
        let path = Path::new(&path_str);

        let content = fs::read_to_string(&path).map_err(|_| {
            ToyError::new(
                ToyErrorType::MissingFile,
                Some(format!("Could not find module {}", module_name)),
            )
        })?;

        let mut lexer = Lexer::new();
        let tokens = lexer.lex(content)?;

        // Use the module prefix for name mangling
        let module_prefix = module_name.replace(".", "::");
        let mut parser = Parser::with_module_prefix(module_prefix);
        let ast = parser.parse(tokens)?;

        let ctx = Context::create();
        let module = ctx.create_module(module_name);
        let mut generator = Generator::new(&ctx, module);

        generator.compile_to_object(ast, module_name.to_string(), false)?;
        Ok(())
    }

    pub fn generate(&mut self, ast: Vec<Ast>, name: String) -> Result<(), ToyError> {
        let imports: Vec<String> = ast
            .iter()
            .filter_map(|node| {
                if let Ast::ImportStmt(name, _) = node {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for import in &imports {
            self.compile_dependency(import)?;
        }

        self.compile_to_object(ast, name.clone(), true)?;

        let mut link_files = vec![format!("{}.o", name)];
        for import in imports {
            link_files.push(format!("{}.o", import));
        }

        let p_args: Vec<String> = env::args().collect();
        let save_temps = p_args.contains(&"--save-temps".to_string());

        self.linker.link(link_files, name.clone(), save_temps)?;

        if p_args.contains(&"--repl".to_string()) || p_args.contains(&"--run".to_string()) {
            let output_name = format!("{}{}", name, FILE_EXTENSION_EXE);
            let mut prgm = Command::new(format!("{}{}", "./", output_name));
            let _ = prgm
                .spawn()
                .map_err(|e| {
                    ToyError::new(
                        ToyErrorType::LlvmError(format!("Failed to run program: {}", e)),
                        None,
                    )
                })?
                .wait()
                .unwrap();

            if !save_temps {
                let _ = fs::remove_file(&output_name);
                let _ = fs::remove_file(format!("{}.o", name));
                let _ = fs::remove_file(format!("{}.ll", name));
            }
        }
        return Ok(());
    }
}
