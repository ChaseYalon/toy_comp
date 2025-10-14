use crate::debug;
use crate::{
    parser::ast::{Ast, InfixOp},
    token::TypeTok,
};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::env;
use std::fs::File;
use std::io::Write;

pub struct Compiler {
    ast: Vec<Ast>,
    var_count: usize,
    main_scope: Rc<RefCell<Scope>>,
}

#[derive(Debug, Clone, Default)]
struct Scope {
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
        let builder = JITBuilder::new(cranelift_module::default_libcall_names());
        JITModule::new(builder.unwrap())
    }

    fn compile_expr(
        &self,
        expr: &Ast,
        _module: &mut JITModule,
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

    fn compile_var_reassign(
        &mut self,
        var_res: &Ast,
        _module: &mut JITModule,
        builder: &mut FunctionBuilder<'_>,
        scope: &mut Rc<RefCell<Scope>>,
    ) {
        if var_res.node_type() != "VarReassign" {
            panic!("[ERROR] Expecting VarReassign, got {}", var_res);
        }
        let var_name: String;
        let new_val: Ast;
        match var_res {
            Ast::VarReassign(name, new_val_b) => {
                var_name = *name.clone();
                new_val = *new_val_b.clone()
            }
            _ => panic!("[ERROR] Expecting VarReassign, got {}", var_res),
        }
        let (var, _old_type) = scope.as_ref().borrow().get(var_name);
        let var = var; // Copy the variable
        let (val, _) = self.compile_expr(&new_val, _module, builder, scope);
        // Use def_var to update the existing variable instead of creating a new one
        builder.def_var(var, val);
    }

    fn compile_var_dec(
        &mut self,
        var_dec: &Ast,
        _module: &mut JITModule,
        builder: &mut FunctionBuilder<'_>,
        scope: &mut Rc<RefCell<Scope>>,
    ) {
        if var_dec.node_type() != "VarDec" {
            panic!("[ERROR] Expected variable declarations, got {}, of type {}", var_dec, var_dec.node_type());
        }
        let name: String;
        let val: Ast;
        let t_o: &TypeTok;
        match var_dec {
            Ast::VarDec(n, t, v) => {
                name = *n.clone();
                val = *v.clone();
                t_o = t;
            }
            _ => {
                panic!("[ERROR] Expected variable declarations, got {}", var_dec);
            }
        }
        let (val, _) = self.compile_expr(&val, _module, builder, scope);
        let var = Variable::new(self.var_count);
        builder.declare_var(var, types::I64);
        builder.def_var(var, val);
        self.var_count += 1;
        debug!(targets: ["compiler_verbose"], val);
        debug!(targets: ["compiler_verbose"], var);
        scope.borrow_mut().set(name, var, t_o.clone());
    }

    fn compile_if_stmt(
        &mut self,
        node: &Ast,
        _module: &mut JITModule,
        builder: &mut FunctionBuilder<'_>,
        scope: &mut Rc<RefCell<Scope>>,
    ) {
        let (cond_ast, body_asts, alt_op) = match node {
            Ast::IfStmt(cond, body, alt) => (cond, body, alt),
            _ => panic!("[ERROR] Expected IfStmt node, got {:?}", node),
        };

        let (cond_val, _cond_type) = self.compile_expr(&cond_ast, _module, builder, scope);

        let then_block = builder.create_block();
        let merge_block = builder.create_block();
        let else_block = builder.create_block();

        // Branch: if condition is non-zero, go to then_block, otherwise go to merge_block
        builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

        // Then block
        builder.switch_to_block(then_block);
        builder.seal_block(then_block);

        for stmt in body_asts {
            self.compile_stmt(stmt.clone(), _module, builder, scope);
        }

        builder.ins().jump(merge_block, &[]);

        // ELse block
        builder.switch_to_block(else_block);
        builder.seal_block(else_block);
        if alt_op.is_some() {
            for stmt in alt_op.as_ref().unwrap(){
                self.compile_stmt(stmt.clone(), _module, builder, scope);
            }
        }
        builder.ins().jump(merge_block, &[]);

        // Merge block
        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);
    }

    fn compile_stmt(
        &mut self,
        node: Ast,
        _module: &mut JITModule,
        func_builder: &mut FunctionBuilder<'_>,
        scope: &mut Rc<RefCell<Scope>>,
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
            let mut child_scope = Scope::new_child(scope.clone()); // child points to parent
            self.compile_if_stmt(&node, _module, func_builder, &mut child_scope);
        }

        last_val
    }

    pub fn compile(&mut self, ast: Vec<Ast>) -> fn() -> i64 {
        self.ast = ast.clone();
        let mut module = self.make_jit();
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
        let mut sudo_main_scope =std::mem::take(&mut self.main_scope);
        for node in ast {
            last_val = self.compile_stmt(node, &mut module, &mut func_builder, &mut sudo_main_scope);
        }

        self.main_scope = sudo_main_scope;
        
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
        let _ = module.finalize_definitions();

        let code_ptr = module.get_finalized_function(func_id);
        unsafe { std::mem::transmute::<_, fn() -> i64>(code_ptr) }
    }
}

#[cfg(test)]
mod tests;