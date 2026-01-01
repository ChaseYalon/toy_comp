mod ctla;
mod llvm;
mod tir;
mod linker;
use crate::codegen::linker::Linker;
use crate::codegen::llvm::LlvmGenerator;
use crate::parser::ast::Ast;
use crate::{codegen::ctla::CTLA, errors::ToyError};
use crate::errors::ToyErrorType;
use crate::lexer::Lexer;
use crate::parser::Parser;
use inkwell::context::Context;
use inkwell::module::Module;
use std::fs;
use tir::AstToIrConverter;
pub use tir::ir::{Block, Function, SSAValue, TIR, TirType};

pub struct Generator<'a> {
    converter: AstToIrConverter,
    analyzer: CTLA,
    generator: LlvmGenerator<'a>,
    linker: Linker
}

impl<'a> Generator<'a> {
    pub fn new(ctx: &'a Context, main_module: Module<'a>) -> Generator<'a> {
        Generator {
            converter: AstToIrConverter::new(),
            analyzer: CTLA::new(),
            generator: LlvmGenerator::new(ctx, main_module), //I hate that that is nesscary
            linker: Linker::new()
        }
    }

    pub fn compile_to_object(&mut self, ast: Vec<Ast>, name: String, is_main: bool) -> Result<(), ToyError> {
        let _ = self.converter.convert(ast, is_main)?;
        let ir = self.analyzer.analyze(self.converter.builder.clone())?;
        self.generator.generate(ir, name)?;
        Ok(())
    }

    fn compile_dependency(&self, module_name: &str) -> Result<(), ToyError> {
        let path = format!("std/{}.toy", module_name);
        let content = fs::read_to_string(&path).map_err(|_| {
            ToyError::new(
                ToyErrorType::MissingFile,
                Some(format!("Could not find module {}", module_name)),
            )
        })?;

        let mut lexer = Lexer::new();
        let tokens = lexer.lex(content)?;

        let mut parser = Parser::new();
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

        self.linker.link(link_files, name)?;
        return Ok(());
    }
}
