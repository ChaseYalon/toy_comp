use crate::debug;
use crate::{
    parser::ast::{Ast, InfixOp},
    token::TypeTok,
};
use cranelift::prelude::*;
use cranelift_codegen::Context;
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, FuncId};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

mod vars;
unsafe extern "C" {
    fn toy_print(input: i64, datatype: i64);
    fn toy_println(input: i64, datatype: i64);
    fn toy_malloc(ptr: i64) -> i64;
    fn toy_concat(sp1: i64, sp2: i64) -> i64;
    fn toy_strequal(sp1: i64, sp2: i64) -> i64;
    fn toy_strlen(sp1: i64) -> i64;
}

pub enum OutputType {
    Jit(JITModule),
    Aot(ObjectModule),
}
pub static STUB_C: &str = include_str!("../c/stub.c");
pub static BUILTIN_C: &str = include_str!("../c/builtins.c");
pub struct Compiler {
    ast: Vec<Ast>,
    var_count: usize,
    main_scope: Rc<RefCell<Scope>>,
    funcs: HashMap<String, (TypeTok, FuncId)>,
    func_ir: Vec<String>,
    loop_cond_block: Option<Block>,
    loop_merge_block: Option<Block>,
}

#[derive(Debug, Clone, Default)]
pub struct Scope {
    vars: HashMap<String, (Variable, TypeTok)>,
    parent: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
    fn new_child(parent: &Rc<RefCell<Scope>>) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent: Some(parent.clone()),
        }))
    }

    fn set(&mut self, name: String, val: Variable, ty: TypeTok) {
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
            funcs: HashMap::new(),
            func_ir: Vec::new(),
            loop_cond_block: None,
            loop_merge_block: None,
        }
    }

    fn make_jit(&self) -> JITModule {
        let mut jit_builder = JITBuilder::new(default_libcall_names()).unwrap();
        jit_builder.symbol("toy_print", toy_print as *const u8);
        jit_builder.symbol("toy_println", toy_println as *const u8);
        jit_builder.symbol("toy_malloc", toy_malloc as *const u8);
        jit_builder.symbol("toy_concat", toy_concat as *const u8);
        jit_builder.symbol("toy_strequal", toy_strequal as *const u8);
        jit_builder.symbol("toy_strlen", toy_strlen as *const u8);
        JITModule::new(jit_builder)
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
    fn declare_builtin_funcs<M: Module>(&mut self, module: &mut M) {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        //Toy malloc takes a pointer to a string and allocates it in memory, returning the pointer to that allocation
        let func = module
            .declare_function("toy_malloc", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("malloc".to_string(), (TypeTok::Int, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_print", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("print".to_string(), (TypeTok::Void, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_println", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("println".to_string(), (TypeTok::Void, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); //str a
        sig.params.push(AbiParam::new(types::I64)); //str b
        sig.returns.push(AbiParam::new(types::I64)); //Ptr to a + b
        let func = module
            .declare_function("toy_concat", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("concat".to_string(), (TypeTok::Int, func)); //Returns a pointer to the new string

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_strequal", Linkage::Import, &sig)
            .unwrap();
        self.funcs
            .insert("strequal".to_string(), (TypeTok::Bool, func));

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func = module
            .declare_function("toy_strlen", Linkage::Import, &sig)
            .unwrap();
        self.funcs.insert("len".to_string(), (TypeTok::Int, func));
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

        builder
            .ins()
            .brif(cond_val, then_block, &[], else_block, &[]);

        builder.switch_to_block(then_block);
        builder.seal_block(then_block);

        let then_scope = Scope::new_child(scope);
        let mut then_has_terminator = false;
        for stmt in body_asts {
            if matches!(stmt, Ast::Break | Ast::Continue) {
                self.compile_stmt(stmt.clone(), _module, builder, &then_scope);
                then_has_terminator = true;
                break;
            }

            self.compile_stmt(stmt.clone(), _module, builder, &then_scope);

            if let Some(current_block) = builder.current_block() {
                if let Some(last_inst) = builder.func.layout.last_inst(current_block) {
                    if builder.func.dfg.insts[last_inst].opcode().is_terminator() {
                        then_has_terminator = true;
                        break;
                    }
                }
            }
        }

        if !then_has_terminator {
            builder.ins().jump(merge_block, &[]);
        }

        builder.switch_to_block(else_block);
        builder.seal_block(else_block);

        let mut else_has_terminator = false;
        if let Some(alt_stmts) = alt_op {
            let else_scope = Scope::new_child(scope);
            for stmt in alt_stmts {
                if matches!(stmt, Ast::Break | Ast::Continue) {
                    self.compile_stmt(stmt.clone(), _module, builder, &else_scope);
                    else_has_terminator = true;
                    break;
                }

                self.compile_stmt(stmt.clone(), _module, builder, &else_scope);

                if let Some(current_block) = builder.current_block() {
                    if let Some(last_inst) = builder.func.layout.last_inst(current_block) {
                        if builder.func.dfg.insts[last_inst].opcode().is_terminator() {
                            else_has_terminator = true;
                            break;
                        }
                    }
                }
            }
        }

        if !else_has_terminator {
            builder.ins().jump(merge_block, &[]);
        }

        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);
    }
    fn compile_func_dec<M: Module>(
        &mut self,
        node: Ast,
        _module: &mut M,
        scope: &Rc<RefCell<Scope>>,
    ) {
        let mut sig = _module.make_signature();
        let (name, params, return_type, body) = match node {
            Ast::FuncDec(n, p, c, b) => (*n, p, c, b),
            _ => unreachable!(),
        };
        let types: Vec<TypeTok> = params
            .clone()
            .iter()
            .filter_map(|ast| {
                if let Ast::FuncParam(_, t) = ast {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect();

        for _t in types {
            //Right now everything is an int (either bool or int, but both represented as int)
            sig.params.push(AbiParam::new(types::I64));
        }
        if return_type != TypeTok::Void {
            //Again it is either a bool or an int, both represented as i64
            sig.returns.push(AbiParam::new(types::I64));
        }

        //Cranelift stuff
        let func_id = _module
            .declare_function(&name, Linkage::Local, &sig)
            .unwrap();
        self.funcs.insert(name.clone(), (return_type, func_id));
        let mut ctx = _module.make_context();
        ctx.func.signature = sig;
        let mut builder_ctx = FunctionBuilderContext::new();
        let mut func_builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
        let entry_block = func_builder.create_block();
        func_builder.append_block_params_for_function_params(entry_block);
        func_builder.switch_to_block(entry_block);
        func_builder.seal_block(entry_block);

        let func_scope = Scope::new_child(scope);

        let block_params: Vec<Value> = func_builder.block_params(entry_block).to_vec();
        for (i, param) in params.iter().enumerate() {
            match param {
                Ast::FuncParam(param_name, param_type) => {
                    let var = Variable::new(self.var_count);
                    self.var_count += 1;

                    func_builder.declare_var(var, types::I64);

                    func_builder.def_var(var, block_params[i]);

                    func_scope
                        .borrow_mut()
                        .set((**param_name).clone(), var, param_type.clone());
                }
                _ => panic!("[ERROR] Expected FuncParam, got {}", param),
            }
        }

        for stmt in body {
            let _ = self.compile_stmt(stmt, _module, &mut func_builder, &func_scope);
        }

        _module.define_function(func_id, &mut ctx).unwrap();
        let args: Vec<String> = env::args().collect();
        if args.contains(&"--save-ir".to_string()) {
            let str = ctx.func.display();
            self.func_ir.push(format!("{}", str));
        }
        _module.clear_context(&mut ctx);
    }
    fn compile_while_stmt<M: Module>(
        &mut self,
        node: &Ast,
        _module: &mut M,
        func_builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) {
        let (cond, body) = match node.clone() {
            Ast::WhileStmt(c, b) => (*c, b),
            _ => unreachable!(),
        };

        let cond_block = func_builder.create_block();
        let body_block = func_builder.create_block();
        let merge_block = func_builder.create_block();

        let prev_cond = self.loop_cond_block;
        let prev_merge = self.loop_merge_block;

        self.loop_cond_block = Some(cond_block);
        self.loop_merge_block = Some(merge_block);

        func_builder.ins().jump(cond_block, &[]);

        func_builder.switch_to_block(cond_block);
        let (v, t) = self.compile_expr(&cond, _module, func_builder, scope);
        if t != TypeTok::Bool {
            panic!("[ERROR] While statement must have boolean expression");
        }
        func_builder
            .ins()
            .brif(v, body_block, &[], merge_block, &[]);

        func_builder.switch_to_block(body_block);
        let child_scope = Scope::new_child(scope);

        for stmt in body {
            if let Some(current_block) = func_builder.current_block() {
                if let Some(last_inst) = func_builder.func.layout.last_inst(current_block) {
                    if func_builder.func.dfg.insts[last_inst]
                        .opcode()
                        .is_terminator()
                    {
                        break;
                    }
                }
            }

            debug!(targets: ["compiler", "compiler_verbose"], format!("Current stmt {}", stmt));
            self.compile_stmt(stmt, _module, func_builder, &child_scope);
        }

        if let Some(current_block) = func_builder.current_block() {
            if let Some(last_inst) = func_builder.func.layout.last_inst(current_block) {
                if !func_builder.func.dfg.insts[last_inst]
                    .opcode()
                    .is_terminator()
                {
                    func_builder.ins().jump(cond_block, &[]);
                }
            } else {
                func_builder.ins().jump(cond_block, &[]);
            }
        }

        func_builder.switch_to_block(merge_block);
        func_builder.seal_block(cond_block);
        func_builder.seal_block(body_block);
        func_builder.seal_block(merge_block);

        self.loop_cond_block = prev_cond;
        self.loop_merge_block = prev_merge;
    }

    fn compile_stmt<M: Module>(
        &mut self,
        node: Ast,
        _module: &mut M,
        func_builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Option<(Value, TypeTok)> {
        debug!(targets: ["compiler_verbose"], "in compile stmt");
        let mut last_val = None;

        if node.node_type() == "Break" {
            if let Some(merge_block) = self.loop_merge_block {
                func_builder.ins().jump(merge_block, &[]);
            } else {
                panic!("[ERROR] Break statement outside of loop");
            }
            return None;
        }

        if node.node_type() == "Continue" {
            if let Some(cond_block) = self.loop_cond_block {
                func_builder.ins().jump(cond_block, &[]);
            } else {
                panic!("[ERROR] Continue statement outside of loop");
            }
            return None;
        }
        println!("Stmt: {}", node);
        if node.node_type() == "IntLit"
            || node.node_type() == "InfixExpr"
            || node.node_type() == "VarRef"
            || node.node_type() == "BoolLit"
            || node.node_type() == "FuncCall"
            || node.node_type() == "StrLit"
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
            let child_scope = Scope::new_child(&scope);
            self.compile_if_stmt(&node, _module, func_builder, &child_scope);
        }

        if node.node_type() == "FuncDec" {
            let child_scope = Scope::new_child(&scope);
            self.compile_func_dec(node.clone(), _module, &child_scope);
        }

        if node.clone().node_type() == "EmptyExpr" {
            let child = match node.clone() {
                Ast::EmptyExpr(chi) => *chi,
                _ => panic!("[ERROR] Expected EmptyExpr, got {}", node),
            };
            self.compile_expr(&child, _module, func_builder, scope);
        }

        if node.node_type() == "Return" {
            let expr = match node.clone() {
                Ast::Return(v) => *v,
                _ => unreachable!(),
            };
            let (val, _) = self.compile_expr(&expr, _module, func_builder, scope);
            func_builder.ins().return_(&[val]);
        }

        if node.node_type() == "WhileStmt" {
            self.compile_while_stmt(&node, _module, func_builder, scope);
        }

        last_val
    }

    fn compile_internal<M: Module>(&mut self, module: &mut M, ast: Vec<Ast>) -> (FuncId, Context) {
        let mut ctx: Context = module.make_context();

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

        self.declare_builtin_funcs(module);

        let sudo_main_scope = self.main_scope.clone();
        for node in ast {
            last_val = self.compile_stmt(node, module, &mut func_builder, &sudo_main_scope);
        }

        debug!(targets: ["compiler_verbose"], format!("Last val: {:?}", last_val));
        let (ret_val, _) =
            last_val.unwrap_or_else(|| (func_builder.ins().iconst(types::I64, 0), TypeTok::Int));
        func_builder.ins().return_(&[ret_val]);

        func_builder.finalize();

        let args: Vec<String> = env::args().collect();

        let func_id = module
            .declare_function("user_main", Linkage::Export, &ctx.func.signature)
            .unwrap();

        module.define_function(func_id, &mut ctx).unwrap();
        if args.contains(&"--save-ir".to_string()) {
            let str = format!("{}", ctx.func.display());
            self.func_ir.push(str);
            let mut ir: String = String::new();
            for s in self.func_ir.clone() {
                ir += &s;
            }
            let mut file = File::create("ir.clif").unwrap();
            file.write_all(ir.as_bytes()).unwrap();
        }
        module.clear_context(&mut ctx);

        (func_id, ctx)
    }

    pub fn compile(
        &mut self,
        ast: Vec<Ast>,
        should_jit: bool,
        path: Option<&str>,
    ) -> Option<fn() -> i64> {
        if !should_jit {
            let o_path = path.unwrap_or("program.exe");

            let base_name = Path::new(o_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("program");
            let obj_temp = format!("{}.obj", base_name);
            let stub_temp = format!("{}_stub.c", base_name);
            let builtin_temp = format!("{}_builtins.c", base_name);
            let obj_path = Path::new(&obj_temp);
            let stub_path = Path::new(&stub_temp);
            let builtin_path = Path::new(&builtin_temp);

            let mut obj_file = File::create(&obj_path).unwrap();
            obj_file
                .write_all(&self.compile_to_object(ast.clone()))
                .unwrap();

            let mut stub_file = File::create(&stub_path).unwrap();
            stub_file.write_all(STUB_C.as_bytes()).unwrap();

            let mut builtin_file = File::create(&builtin_path).unwrap();
            builtin_file.write_all(BUILTIN_C.as_bytes()).unwrap();

            let status = Command::new("gcc")
                .args(&[
                    obj_path.to_str().unwrap(),
                    stub_path.to_str().unwrap(),
                    builtin_path.to_str().unwrap(),
                    "-o",
                    o_path,
                ])
                .status()
                .expect("failed to execute gcc");

            if !status.success() {
                panic!("GCC failed with exit code {:?}", status.code());
            }
            //remove c objs
            let _ = std::fs::remove_file(stub_path);
            let _ = std::fs::remove_file(builtin_path);
            let args: Vec<String> = env::args().collect();
            if !args.contains(&"--save-temp".to_string()) {
                let _ = std::fs::remove_file(obj_path);
            }

            return None;
        }
        self.ast = ast.clone();
        let mut module = self.make_jit();

        let (func_id, _ctx) = self.compile_internal(&mut module, ast);

        module.finalize_definitions().unwrap();

        let code_ptr = module.get_finalized_function(func_id);
        return Some(unsafe { std::mem::transmute::<_, fn() -> i64>(code_ptr) });
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
