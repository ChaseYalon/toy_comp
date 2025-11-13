use super::{Compiler, Scope};
use crate::debug;
use crate::parser::ast::Ast;
use crate::token::TypeTok;
use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
impl Compiler {
    fn compile_struct_reassign<M: Module>(
        &mut self,
        node: &Ast,
        module: &mut M,
        func_builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) {
        let (name, fields, value) = match node.clone() {
            Ast::StructReassign(n, f, v) => (*n, f, *v),
            _ => unreachable!(),
        };

        let (parent_struct_interface_name, parent_struct_ptr_var) =
            scope.borrow().get_struct(name.clone());
        let parent_struct = scope.borrow().get_interface(parent_struct_interface_name);
        let parent_struct_ptr = func_builder.use_var(parent_struct_ptr_var);
        let mut final_type: TypeTok = TypeTok::Any; // default, placeholder
        let mut current_struct: HashMap<String, Box<TypeTok>> = parent_struct
            .iter()
            .map(|(k, v)| (k.clone(), Box::new(v.clone())))
            .collect();

        for (i, field) in fields.iter().enumerate() {
            let boxed_field = current_struct.get(field).expect(&format!(
                "[ERROR] Field {} does not exist in struct {}",
                field, name
            ));

            if i == fields.len() - 1 {
                final_type = (**boxed_field).clone();
            } else {
                current_struct = match &**boxed_field {
                    TypeTok::Struct(m) => m
                        .iter()
                        .map(|(k, v)| (k.clone(), Box::new(*(v.clone()))))
                        .collect(),
                    _ => panic!("[ERROR] Variable {} is not a struct", field),
                };
            }
        }

        let (new_val, new_val_type) = self.compile_expr(&value, module, func_builder, scope);
        if new_val_type != final_type {
            panic!(
                "[ERROR] Expected value of type {:?}, got value of type {:?}",
                final_type, new_val_type
            );
        }

        let (first_field, _) = self.compile_expr(
            &Ast::StringLit(Box::new(fields[0].clone())),
            module,
            func_builder,
            scope,
        );
        let (_, toy_put_global, _) = self.funcs.get("toy_put").unwrap();
        let toy_put = module.declare_func_in_func(*toy_put_global, &mut func_builder.func);
        func_builder
            .ins()
            .call(toy_put, &[parent_struct_ptr, first_field, new_val]);
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
        self.is_in_func = true;
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
        let names: Vec<String> = params
            .clone()
            .iter()
            .filter_map(|ast| {
                if let Ast::FuncParam(t, _) = ast {
                    Some(*t.clone())
                } else {
                    None
                }
            });
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
        self.funcs.insert(name.clone(), (return_type, func_id, names));
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
            if let Ast::FuncParam(param_name, param_type) = param {
                let var = Variable::new(self.var_count);
                self.var_count += 1;

                func_builder.declare_var(var, types::I64);
                func_builder.def_var(var, block_params[i]);
                func_scope
                    .borrow_mut()
                    .set(*param_name.clone(), var, param_type.clone());

                if let TypeTok::Struct(map) = param_type {
                    let unboxed: HashMap<String, TypeTok> = map
                        .clone()
                        .iter()
                        .map(|(k, v)| (k.clone(), *v.clone()))
                        .collect();
                    /*
                        Some notes on where I lef off
                            get_struct has been modified to return a Variable instead of a value
                            this means that toy_malloc can be passed a variable
                            Down here you have to set f to a "dummy value"
                            then you have to modify the func_call to when it receives a value of struct type allocate it and reassign the variable
                     */
                    let (n , _) = self.compile_expr(&Ast::IntLit(-1), _module, &mut func_builder, &func_scope);
                    func_builder.declare_var(Variable::new(self.var_count), types::I64);
                    func_builder.def_var(Variable::new(self.var_count), n);
                    scope.borrow_mut().set_unresolved_struct(
                        *param_name.clone(),
                        unboxed,
                        Variable::new(self.var_count),
                    );
                    self.var_count += 1;
                }
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
        self.is_in_func = false;
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
    fn compile_struct_interface<M: Module>(
        &mut self,
        node: Ast,
        _module: &mut M,
        _builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) -> Option<(Value, TypeTok)> {
        let (name, kv) = match node.clone() {
            Ast::StructInterface(n, kv) => (*n, *(kv).clone()),
            _ => unreachable!(),
        };
        scope.borrow_mut().set_interface(name, kv.clone());
        return None;
    }
    pub fn compile_stmt<M: Module>(
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
        if node.node_type() == "IntLit"
            || node.node_type() == "InfixExpr"
            || node.node_type() == "VarRef"
            || node.node_type() == "BoolLit"
            || node.node_type() == "FuncCall"
            || node.node_type() == "StrLit"
            || node.node_type() == "FloatLit"
            || node.node_type() == "ArrLit"
            || node.node_type() == "ArrRef"
            || node.node_type() == "StructLit"
            || node.node_type() == "StructRef"
        {
            last_val = Some(self.compile_expr(&node, _module, func_builder, scope));
        }

        if node.node_type() == "VarDec" {
            debug!(targets: ["compiler_verbose"], format!("Node: {}", node));
            self.compile_var_dec(&node, _module, func_builder, scope);
        }

        if node.node_type() == "VarReassign" {
            self.compile_var_reassign(&node, _module, func_builder, scope);
        }

        if node.node_type() == "IfStmt" {
            let child_scope = Scope::new_child(scope);
            self.compile_if_stmt(&node, _module, func_builder, &child_scope);
        }

        if node.node_type() == "FuncDec" {
            let child_scope = Scope::new_child(scope);
            self.compile_func_dec(node.clone(), _module, &child_scope);
        }
        if node.node_type() == "ArrReassign" {
            let (a, i, v) = match &node {
                Ast::ArrReassign(aa, ii, vv) => (*aa.clone(), ii.clone(), *vv.clone()),
                _ => unreachable!(),
            };
            let (_, arr_write_global, _) = self.funcs.get("toy_write_to_arr").unwrap();
            let arr_write = _module.declare_func_in_func(*arr_write_global, &mut func_builder.func);
            let (idx, _) = self.compile_expr(&i[0], _module, func_builder, scope);
            let (val, t) = self.compile_expr(&v, _module, func_builder, scope);
            let (arr_v, _) = scope.as_ref().borrow().get(a);
            let arr = func_builder.use_var(arr_v);
            let mut params = [arr, val, idx].to_vec();
            self.inject_type_param(&t, false, _module, func_builder, &mut params);
            func_builder.ins().call(arr_write, params.as_slice());
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
        if node.node_type() == "StructInterface" {
            self.compile_struct_interface(node.clone(), _module, func_builder, scope);
        }
        if node.node_type() == "StructReassign" {
            self.compile_struct_reassign(&node, _module, func_builder, scope);
        }
        last_val
    }
}
