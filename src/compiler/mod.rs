use crate::debug;
use crate::{
    parser::ast::{Ast, InfixOp},
    token::TypeTok,
};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_codegen::isa;
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_module::FuncId;
use cranelift_codegen::Context;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::env;
use std::fs::File;
use std::io::Write;

mod vars;

pub enum OutputType {
    Jit(JITModule),
    Aot(ObjectModule)
}

pub struct Compiler {
    ast: Vec<Ast>,
    var_count: usize,
    main_scope: Rc<RefCell<Scope>>,
}

#[derive(Debug, Clone, Default)]
pub struct Scope {
    vars: HashMap<String, (Variable, TypeTok)>,
    parent: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
    fn new_child(parent: Rc<RefCell<Scope>>) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent: Some(parent.clone()),
        }))
    }
    
    fn set(&mut self, name: String, val: Variable, ty: TypeTok){
        self.vars.insert(name, (val, ty));
    }
    
    fn get(&self, name: String) -> (Variable, TypeTok) {
        if self.vars.contains_key(&name) {
            return self.vars.get(&name).unwrap().clone();
        }
        if self.parent.is_none() {
            panic!("[ERROR] Variable \"{}\" is undefined", name);
        }
        return self.parent.as_ref().unwrap().borrow().get(name);
    }
}


impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            ast: Vec::new(),
            var_count: 0,
            main_scope: Rc::new(RefCell::new(Scope {
                vars: HashMap::new(),
                parent: None,
            })),
        }
    }
    
    fn make_jit(&self) -> JITModule {
        let jit_builder = JITBuilder::new(default_libcall_names());
        JITModule::new(jit_builder.unwrap())
    }
    
    fn make_object(&self) -> ObjectModule {
        let triple = Triple::host();
        let isa_builder = isa::lookup(triple).expect("ISA lookup failed");
        
        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "true").unwrap();
        let flags = settings::Flags::new(flag_builder);
        
        let isa = isa_builder.finish(flags).expect("Failed to finish ISA");
        
        let obj_builder = ObjectBuilder::new(isa, "toy_lang".to_string(), default_libcall_names())
            .expect("ObjectBuilder creation failed");
        ObjectModule::new(obj_builder)
    }

    fn compile_expr<M: Module>(
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
        {
            panic!("[ERROR] Unknown AST node type: {}", expr.node_type());
        }

        match expr {
            Ast::IntLit(n) => (builder.ins().iconst(types::I64, *n), TypeTok::Int),
            Ast::BoolLit(b) => {
                let is_true: i64 = if *b { 1 } else { 0 };
                (builder.ins().iconst(types::I64, is_true), TypeTok::Bool)
            }
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


    fn compile_if_stmt<M: Module>(
        &mut self,
        node: &Ast,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) {
        let (cond_ast, body_asts, alt_op) = match node {
            Ast::IfStmt(cond, body, alt) => (cond, body, alt),
            _ => panic!("[ERROR] Expected IfStmt node, got {:?}", node),
        };

        let (cond_val, _cond_type) = self.compile_expr(&cond_ast, _module, builder, scope);

        let then_block = builder.create_block();
        let else_block = builder.create_block();
        let merge_block = builder.create_block();

        builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

        // Then block
        builder.switch_to_block(then_block);
        builder.seal_block(then_block);

        for stmt in body_asts {
            self.compile_stmt(stmt.clone(), _module, builder, scope);
        }

        builder.ins().jump(merge_block, &[]);

        // Else block
        builder.switch_to_block(else_block);
        builder.seal_block(else_block);
        
        if let Some(alt_stmts) = alt_op {
            for stmt in alt_stmts {
                self.compile_stmt(stmt.clone(), _module, builder, scope);
            }
        }
        builder.ins().jump(merge_block, &[]);

        // Merge block
        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);
    }

    fn compile_stmt<M: Module>(
        &mut self,
        node: Ast,
        _module: &mut M,
        func_builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Option<(Value, TypeTok)> {
        let mut last_val = None;

        debug!(targets: ["compiler"], format!("compile_stmt: node_type={}", node.node_type()).as_str());

        if node.node_type() == "IntLit"
            || node.node_type() == "InfixExpr"
            || node.node_type() == "VarRef"
            || node.node_type() == "BoolLit"
        {
            last_val = Some(self.compile_expr(&node, _module, func_builder, scope));
        }

        if node.node_type() == "VarDec" {
            self.compile_var_dec(&node, _module, func_builder, scope);
        }

        if node.node_type() == "VarReassign" {
            self.compile_var_reassign(&node, _module, func_builder, scope);
        }

        if node.node_type() == "IfStmt" {
            let child_scope = Scope::new_child(scope.clone());
            self.compile_if_stmt(&node, _module, func_builder, &child_scope);
        }

        last_val
    }

    fn compile_internal<M: Module>(
        &mut self,
        module: &mut M,
        ast: Vec<Ast>,
    ) -> (FuncId, Context) {
        let mut ctx = module.make_context();

        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(types::I64));
        ctx.func.signature = sig;

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut func_builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

        let main_block = func_builder.create_block();
        func_builder.switch_to_block(main_block);
        func_builder.append_block_params_for_function_params(main_block);
        func_builder.seal_block(main_block);

        let mut last_val: Option<(Value, TypeTok)> = None;
        let sudo_main_scope = self.main_scope.clone();
        for node in ast {
            last_val = self.compile_stmt(node, module, &mut func_builder, &sudo_main_scope);
        }

        let (ret_val, _) =
            last_val.unwrap_or_else(|| (func_builder.ins().iconst(types::I64, 0), TypeTok::Int));
        func_builder.ins().return_(&[ret_val]);

        func_builder.finalize();

        let args: Vec<String> = env::args().collect();
        if args.contains(&"--save-ir".to_string()) {
            let ir = format!("{}", ctx.func.display());
            let mut file = File::create("ir.clif").unwrap();
            file.write_all(ir.as_bytes()).unwrap();
        }

        let func_id = module
            .declare_function("main", Linkage::Export, &ctx.func.signature)
            .unwrap();

        module.define_function(func_id, &mut ctx).unwrap();
        module.clear_context(&mut ctx);
        
        (func_id, ctx)
    }

    pub fn compile(&mut self, ast: Vec<Ast>, should_jit: bool, path: Option<&str>) -> Option<fn() -> i64> {
        if !should_jit {
            let mut file = File::create(path.unwrap()).unwrap();
            file.write_all(&self.compile_to_object(ast.clone())).unwrap();
            return None;
        }
        self.ast = ast.clone();
        let mut module = self.make_jit();
        
        let (func_id, _ctx) = self.compile_internal(&mut module, ast);
        
        module.finalize_definitions().unwrap();

        let code_ptr = module.get_finalized_function(func_id);
        return Some(unsafe { std::mem::transmute::<_, fn() -> i64>(code_ptr) })
    }

    fn compile_to_object(&mut self, ast: Vec<Ast>) -> Vec<u8> {
        self.ast = ast.clone();
        let mut module = self.make_object();
        
        let (_func_id, _ctx) = self.compile_internal(&mut module, ast);
        
        let object_product = module.finish();
        object_product.emit().unwrap()
    }
}

#[cfg(test)]
mod tests;