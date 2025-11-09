use super::Compiler;
use crate::debug;
use crate::parser::ast::Ast;
use crate::token::TypeTok;
use cranelift::prelude::*;
use cranelift_module::Module;
#[allow(unused_imports)] //Used for debugging
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

impl Compiler {
    pub fn compile_var_reassign<M: Module>(
        &mut self,
        var_res: &Ast,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
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
        let (val, _) = self.compile_expr(&new_val, _module, builder, scope);
        builder.def_var(var, val);
    }

    pub fn compile_var_dec<M: Module>(
        &mut self,
        var_dec: &Ast,
        _module: &mut M,
        builder: &mut FunctionBuilder<'_>,
        scope: &Rc<RefCell<Scope>>,
    ) {
        if var_dec.node_type() != "VarDec" {
            panic!(
                "[ERROR] Expected variable declarations, got {}, of type {}",
                var_dec,
                var_dec.node_type()
            );
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
        if t_o.type_str() == "Struct" {
            self.current_struct_name = Some(name.clone());
        }
        let (val, _) = self.compile_expr(&val, _module, builder, scope);
        let var = Variable::new(self.var_count);
        debug!(targets: ["compiler_verbose"], format!("Value: {:?}", val.type_id()));
        debug!(targets: ["compiler_verbose"], format!("Value {:?}", var.type_id()));
        builder.declare_var(var, types::I64);
        builder.def_var(var, val);
        self.var_count += 1;
        scope.borrow_mut().set(name, var, t_o.clone());
    }
}
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub vars: HashMap<String, (Variable, TypeTok)>,
    pub parent: Option<Rc<RefCell<Scope>>>,
    pub interfaces: HashMap<String, HashMap<String, TypeTok>>,
    ///the value is a pointer to a ToyHashMap, string is the interface name
    pub structs: HashMap<String, (String, Value)>,
}

impl Scope {
    pub fn new_child(parent: &Rc<RefCell<Scope>>) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent: Some(parent.clone()),
            interfaces: HashMap::new(),
            structs: HashMap::new(),
        }))
    }

    pub fn set(&mut self, name: String, val: Variable, ty: TypeTok) {
        self.vars.insert(name, (val, ty));
    }

    pub fn get(&self, name: String) -> (Variable, TypeTok) {
        if self.vars.contains_key(&name) {
            return self.vars.get(&name).unwrap().clone();
        }
        if self.parent.is_none() {
            panic!("[ERROR] Variable \"{}\" is undefined", name);
        }
        return self.parent.as_ref().unwrap().borrow().get(name);
    }
    pub fn set_interface(&mut self, name: String, val: HashMap<String, TypeTok>) {
        self.interfaces.insert(name, val);
    }
    pub fn get_interface(&self, name: String) -> HashMap<String, TypeTok> {
        if self.interfaces.contains_key(&name) {
            return self.interfaces.get(&name).unwrap().clone();
        }
        if self.parent.is_none() {
            panic!("[ERROR] Interface \"{}\" does not exist", name);
        }
        return self.parent.as_ref().unwrap().borrow().get_interface(name);
    }
    pub fn set_struct(&mut self, name: String, val: String, ptr: Value) {
        self.structs.insert(name, (val, ptr));
    }
    pub fn get_struct(&self, name: String) -> (String, Value) {
        if self.structs.contains_key(&name) {
            return self.structs.get(&name).unwrap().clone();
        }
        if self.parent.is_none() {
            panic!("[ERROR] Struct \"{}\" does not exist", name);
        }
        return self.parent.as_ref().unwrap().borrow().get_struct(name);
    }
}
