use inkwell::context::Context;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    path::Path,
    process::Command,
};

use crate::{
    codegen::Generator,
    errors::{ToyError, ToyErrorType},
    lexer::Lexer,
    parser::{ast::Ast, ast_gen::AstGenerator, boxer::Boxer, toy_box::TBox},
    token::TypeTok,
};

pub static FILE_EXTENSION_EXE: &str = if cfg!(target_os = "windows") {
    ".exe"
} else {
    ""
};

pub struct Linker {}

impl Linker {
    pub fn new() -> Linker {
        Linker {}
    }
    pub fn link(
        &mut self,
        files: Vec<String>,
        output: String,
        save_temps: bool,
    ) -> Result<(), ToyError> {
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
        let output_name = format!("{}{}", output, FILE_EXTENSION_EXE);
        let args: Vec<&str> = if env::consts::OS == "windows" {
            let mut args = vec![
                "-m",
                "i386pep",
                crt2_path.to_str().unwrap(),
                crtbegin_path.to_str().unwrap(),
            ];
            for file in &files {
                args.push(file.as_str());
            }
            args.extend_from_slice(&[
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
            ]);
            args
        } else {
            let mut args = vec![
                "-m",
                "elf_x86_64",
                crt1_path.to_str().unwrap(),
                crti_path.to_str().unwrap(),
            ];
            for file in &files {
                args.push(file.as_str());
            }
            args.extend_from_slice(&[
                lbruntime_path.to_str().unwrap(),
                crtn_path.to_str().unwrap(),
                libc_path.to_str().unwrap(),
                libm_path.to_str().unwrap(),
                "-dynamic-linker",
                "/lib64/ld-linux-x86-64.so.2",
                "-o",
                output_name.as_str(),
            ]);
            args
        };
        let rstatus = Command::new(lib_path.join("ld.lld"))
            .args(args.clone())
            .status();
        let status = match rstatus {
            Ok(f) => f,
            Err(_) => {
                eprintln!("Linker args: {:#?}", args);
                return Err(ToyError::new(ToyErrorType::InternalLinkerFailure, None));
            }
        };
        if !status.success() {
            return Err(ToyError::new(ToyErrorType::InternalLinkerFailure, None));
        }
        if save_temps || std::env::var("TOY_DEBUG").unwrap_or("FALSE".to_string()) == "TRUE" {
            return Ok(());
        }
        //kinda hacky
        for file in &files {
            //I dont really care if it failed to remove...
            let _ = fs::remove_file(file);
        }
        Ok(())
    }
}
pub enum ModuleExportType {
    ///in param types(in order declared), return type
    Function(Vec<TypeTok>, TypeTok),
    ///this is for struct interfaces and it contains the TypeTok for the interface
    Struct(TypeTok),
}
pub struct ModuleExport {
    pub name: String,
    pub ty: ModuleExportType,
}
impl ModuleExport {
    pub fn new(name: String, ty: ModuleExportType) -> ModuleExport {
        return ModuleExport { name, ty };
    }
}
pub struct ProjectExportTable {
    pub alias_to_path: HashMap<String, String>,
    //A module can export 3 things, a function, a struct (interface), or a variable
    pub path_to_exports: HashMap<String, Vec<ModuleExport>>,
}
impl ProjectExportTable {
    pub fn new() -> ProjectExportTable {
        return ProjectExportTable {
            alias_to_path: HashMap::new(),
            path_to_exports: HashMap::new(),
        };
    }
}
pub struct Driver {
    pub table: ProjectExportTable,
    ///project name, for now just the name of the main file, defaults to "program"
    pub name: String,
    ///main file path
    pub main_program: String,
    ///represents the name of all modules that have been parsed, to prevent stack overflows with cyclic dependencies
    pub parsed_modules: HashSet<String>,
    ///contains the path of a file to its generated IR
    pub file_path_to_ast: HashMap<String, Vec<Ast>>,
    pub mangled_lookup: HashMap<String, String>,
}

impl Driver {
    pub fn mangle_name(module_prefix: Option<&str>, name: &str, params: &[TypeTok]) -> String {
        let prefixed_name = if let Some(prefix) = module_prefix {
            if prefix.is_empty() {
                name.to_string()
            } else if name.starts_with(prefix) && name[prefix.len()..].starts_with("::") {
                name.to_string()
            } else if name.contains("::") && !name.contains(prefix) {
                // It has a different prefix? Assume fully qualified.
                name.to_string()
            } else {
                format!("{}::{}", prefix, name)
            }
        } else {
            name.to_string()
        };

        if params.is_empty() {
            return prefixed_name;
        }

        let mut final_mangled_name = prefixed_name;
        for t in params {
            final_mangled_name = format!("{}_{}", final_mangled_name, t.type_str().to_lowercase());
        }
        return final_mangled_name;
    }
    pub fn new(prgm: String) -> Driver {
        return Driver {
            table: ProjectExportTable::new(),
            name: "Program".to_string(),
            main_program: prgm,
            parsed_modules: HashSet::new(),
            file_path_to_ast: HashMap::new(),
            mangled_lookup: HashMap::new(),
        };
    }
    #[allow(unused)]
    pub fn new_with_name(prgm: String, name: String) -> Driver {
        return Driver {
            table: ProjectExportTable::new(),
            name: name,
            main_program: prgm,
            parsed_modules: HashSet::new(),
            file_path_to_ast: HashMap::new(),
            mangled_lookup: HashMap::new(),
        };
    }
    fn name_to_path(&self, path: String) -> String {
        let segments: Vec<&str> = path.split(".").collect();
        return segments.join("/") + ".toy";
    }

    fn feed_to_ast_gen(&mut self, ast_gen: &mut AstGenerator) {
        let mut new_lookups: Vec<(String, String)> = Vec::new();

        for (path, exports) in &self.table.path_to_exports {
            let module_name = path
                .replace("/", ".")
                .replace(".toy", "")
                .trim_start_matches('.')
                .to_string();
            let prefix = module_name.replace(".", "::");

            for export in exports {
                match &export.ty {
                    ModuleExportType::Function(params, ret) => {
                        // export.name is already mangled by Boxer (e.g., "abs_int")
                        // Only add module prefix, don't re-add params
                        let full_mangled = Driver::mangle_name(Some(&prefix), &export.name, &[]);
                        ast_gen.register_function(
                            full_mangled.clone(),
                            params.clone(),
                            ret.clone(),
                        );

                        let basic_name = if export.name.contains("::") {
                            export.name.clone()
                        } else {
                            format!("{}::{}", prefix, export.name)
                        };
                        new_lookups.push((full_mangled, basic_name));
                    }
                    ModuleExportType::Struct(ty) => {
                        if let TypeTok::Struct(fields) = ty {
                            let full_name = if export.name.contains("::") {
                                export.name.clone()
                            } else {
                                format!("{}::{}", prefix, export.name)
                            };
                            ast_gen.register_struct(full_name, fields.clone());
                        }
                    }
                }
            }
        }
        for (k, v) in new_lookups {
            self.mangled_lookup.insert(k, v);
        }
    }

    ///Finds and parses all dependencies from a list of TBoxes
    ///Returns a list of paths to import
    fn find_and_parse_dependencies(&mut self, boxes: Vec<TBox>) -> Result<(), ToyError> {
        let mut import_list: Vec<String> = vec![];
        for t_box in boxes {
            match t_box {
                TBox::ImportStmt(import_name, _) => {
                    let path = self.name_to_path(import_name.clone());
                    import_list.push(path.clone());
                    self.table
                        .alias_to_path
                        .insert(path, import_name.split(".").last().unwrap().to_string());
                }
                _ => continue,
            }
        }
        //load lex and box each import
        for import in import_list {
            if self.parsed_modules.contains(&import) {
                //this means module has already been parsed, we can just skip it
                continue;
            }
            self.parsed_modules.insert(import.clone());
            //read the file
            let contents = match fs::read_to_string(import.clone()) {
                Ok(c) => c,
                Err(_) => {
                    return Err(ToyError::new(
                        ToyErrorType::MissingFile,
                        Some(format!("Could not find file: {}", import)),
                    ));
                }
            };

            //create a new lexer and boxer for each module
            let mut l = Lexer::new();
            let import_toks = l.lex(contents)?;
            let module_name = import
                .clone()
                .replace("/", ".")
                .replace(".toy", "")
                .trim_start_matches('.')
                .to_string();
            let prefix = module_name.replace(".", "::");
            let mut b = Boxer::with_module_prefix(prefix.clone());
            let import_boxes = b.box_toks(import_toks)?;

            let mut specific_module_exports: Vec<ModuleExport> = vec![];
            for import_box in import_boxes.clone() {
                match import_box.clone() {
                    //only match if export is set
                    TBox::FuncDec(name, _params, return_type, _, _, true) => {
                        //unwrap is safe because name with always be var(ref/name)
                        let ty = ModuleExportType::Function(
                            import_box.get_func_param_types(),
                            return_type.clone(),
                        );
                        let export = ModuleExport::new(*name.get_var_name().unwrap(), ty);
                        specific_module_exports.push(export);
                    }
                    TBox::ExternFuncDec(name, params, return_type, _) => {
                        let mut param_types = Vec::new();
                        for p in params {
                            if let TBox::FuncParam(_, t, _) = p {
                                param_types.push(t);
                            }
                        }
                        let ty = ModuleExportType::Function(param_types, return_type.clone());
                        let export = ModuleExport::new(*name.get_var_name().unwrap(), ty);
                        specific_module_exports.push(export);
                    }
                    TBox::StructInterface(interface_name, field_map, _) => {
                        let this_is_designed_terribly: BTreeMap<String, Box<TypeTok>> =
                            (*field_map)
                                .clone()
                                .into_iter()
                                .map(|(k, v)| (k, Box::new(v)))
                                .collect();
                        let ty =
                            ModuleExportType::Struct(TypeTok::Struct(this_is_designed_terribly));
                        let export = ModuleExport::new(*interface_name, ty);
                        specific_module_exports.push(export);
                    }
                    _ => continue,
                }
            }
            self.table
                .path_to_exports
                .insert(import.clone(), specific_module_exports);

            //recursively parse dependencies in this imported module
            self.find_and_parse_dependencies(import_boxes.clone())?;

            let mut ast_gen = AstGenerator::with_module_prefix(prefix.clone());
            self.feed_to_ast_gen(&mut ast_gen);
            let ast = ast_gen.generate(import_boxes)?;
            self.file_path_to_ast.insert(import, ast);
        }

        return Ok(());
    }
    ///Starts the main program compilation process
    ///Will automatically compile and build the program
    ///Linking in all nesscary modules
    pub fn start(&mut self, ctx: &Context) -> Result<(), ToyError> {
        //Lex and box main program
        let mut l = Lexer::new();
        let main_prgm_toks = l.lex(self.main_program.clone())?;

        //I am aware this defeats the purpose of the parser meta module
        let mut b = Boxer::new();
        let main_prgm_boxes = b.box_toks(main_prgm_toks)?;

        self.find_and_parse_dependencies(main_prgm_boxes.clone())?;

        let mut ast_gen = AstGenerator::new();
        self.feed_to_ast_gen(&mut ast_gen);
        let main_ast = ast_gen.generate(main_prgm_boxes)?;

        let mut object_files = Vec::new();

        //Compile Dependencies
        for (path, ast) in &self.file_path_to_ast {
            let module_name = path.replace(".toy", "");

            let llvm_module = ctx.create_module(&module_name);
            let mut generator = Generator::new(ctx, llvm_module);
            generator.compile_to_object(ast.clone(), module_name.clone(), false)?;
            object_files.push(format!("{}.o", module_name));
        }

        //Compile Main
        let main_module = ctx.create_module("program");
        let mut generator = Generator::new(ctx, main_module);

        //Register imported functions so TIR knows about them
        for (path, exports) in &self.table.path_to_exports {
            let module_name = path
                .replace("/", ".")
                .replace(".toy", "")
                .trim_start_matches('.')
                .to_string();
            let prefix = module_name.replace(".", "::");
            for export in exports {
                if let ModuleExportType::Function(_, ret) = &export.ty {
                    // export.name is already mangled by Boxer (e.g., "abs_int")
                    // Only add module prefix, don't re-add params
                    let full_mangled = Driver::mangle_name(Some(&prefix), &export.name, &[]);
                    generator.register_imported_func(full_mangled, ret.clone());
                }
            }
        }

        generator.compile_to_object(main_ast, self.name.clone(), true)?;
        object_files.push(format!("{}.o", self.name));

        //Link
        let args = env::args().collect::<Vec<String>>();
        let save_temps = args.contains(&"--save-temps".to_string());
        let mut linker = Linker::new();
        linker.link(object_files, self.name.clone(), save_temps)?;

        Ok(())
    }
    #[allow(unused)]
    pub fn compile_to_ast_from_str(
        &mut self,
        code: String,
        ast_gen: &mut AstGenerator,
    ) -> Result<Vec<Ast>, ToyError> {
        let mut l = Lexer::new();
        let toks = l.lex(code)?;
        let mut b = Boxer::new();
        let boxes = b.box_toks(toks)?;

        self.find_and_parse_dependencies(boxes.clone())?;
        self.feed_to_ast_gen(ast_gen);

        ast_gen.generate(boxes)
    }
}
