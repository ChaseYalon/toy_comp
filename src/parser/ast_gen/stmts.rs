use super::AstGenerator;
use crate::parser::ast::Ast;
use crate::token::{Token, TypeTok};
use std::collections::HashMap;

impl AstGenerator {
    pub fn parse_var_dec(&mut self, input: &Vec<Token>) -> Ast {
      if input[0].tok_type() != "Let" {
          panic!(
              "[ERROR] Variable declaration must start with let, got {}",
              input[0]
          );
        }
        if input[1].tok_type() != "VarName" {
          panic!(
              "[ERROR] Let must be followed by variable name, got {}",
              input[1]
          );
        }
        if input[2].tok_type() != "Assign" && input[2].tok_type() != "Colon" {
          panic!(
              "[ERROR] Variable declaration must have '=' or ':' after name, got {}",
              input[2]
          );
        }
        //right now there is weird behavoir where it ignores they type if it is specified and always uses the expr type
        let has_type = if input[2].tok_type() == "Colon" {2} else {0};
        let name = input[1].clone();
        
        let val = input[3 + has_type..].to_vec();
        println!("Val: {:?}", val);
        let var_type = None; //this is stupid
        if name.tok_type() != "VarName" {
            panic!("[ERROR] Expected variable name, got {}", name);
        }
        let name_str = *name.get_var_name().unwrap();
        let (val_ast, val_type) = self.parse_expr(&val);
        let ret_var_type: TypeTok;
        if var_type.is_some() {
            ret_var_type = var_type.unwrap();
        } else {
            ret_var_type = val_type;
        }
        let node = Ast::VarDec(
            Box::new(name_str.clone()),
            ret_var_type.clone(),
            Box::new(val_ast),
        );
        self.insert_var_type(name_str.clone(), ret_var_type.clone());
        return node;
    }
    pub fn parse_var_reassign(&self, name: Token, value: Vec<Token>) -> Ast {
        let var_node = self.parse_var_ref(&name);
        let (val_node, _) = self.parse_expr(&value);
        return Ast::VarReassign(
            Box::new(match var_node {
                Ast::VarRef(i) => i.to_string(),
                _ => "".to_string(),
            }),
            Box::new(val_node),
        )
    }
}