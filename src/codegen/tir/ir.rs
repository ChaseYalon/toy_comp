#![allow(unused)]

use crate::{
    errors::{ToyError, ToyErrorType},
    parser::ast::InfixOp,
    token::TypeTok,
};
#[derive(PartialEq, Debug, Clone)]

pub enum NumericInfixOp {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
}
#[derive(PartialEq, Debug, Clone)]

pub enum BoolInfixOp {
    GreaterThan,
    LessThan,
    Equals,
    NotEquals,
    GreaterThanEqt,
    LessThenEqt,
    And,
    Or,
}
///randomly generated "handle" that points to an ssa node
pub type ValueId = usize;
pub type BlockId = usize;
#[derive(PartialEq, Debug, Clone)]
pub enum TirType {
    ///used for integers
    I64,
    ///used for booleans
    I1,
    ///floats
    F64,
    ///interfaces are represented as a vec of other types, there are no field names, everything is done by position
    StructInterface(Vec<TirType>),
    Void,
}
#[derive(PartialEq, Debug, Clone)]

pub struct SSAValue {
    pub val: ValueId,
    pub ty: Option<TirType>,
}
#[derive(Debug, PartialEq, Clone)]
pub enum TIR {
    ///value as i64, regardless of weather it is an i64 or i1, and TirType to specify that
    IConst(ValueId, i64, TirType),
    ///numeric infix is any expression that returns a number so 5 + 3, NOT 5 < 3 (see: TBoolInfix)
    ///first valueId is handle for the expression, second and third are left and right, finally InfixOp
    NumericInfix(ValueId, SSAValue, SSAValue, NumericInfixOp),
    ///boolean infix is an expression that "returns" a boolean, can take numeric inputs, so both 5 + 3 and true || false are valid
    ///it is up to the "compiler" to ensure that 5 && 3 is either valid or never happens
    /// first value is the handle for the expression, second and third are left and right, 4th is operator
    BoolInfix(ValueId, SSAValue, SSAValue, BoolInfixOp),
    ///jumps to first block if the ValueId points to an i1 of "1" (true), second "0" (false), compiler will return an error if it is not an i1
    JumpCond(SSAValue, BlockId, BlockId),
    ///jumps unconditionally to AFTER the ValueId provided
    Jump(ValueId),
    ///jumps unconditionally to the BlockId specified
    JumpUnCond(BlockId),
    ///returns the value pointed to, as value if less then the word size, otherwise as a pointer
    Ret(SSAValue),
    ///call locally defined function, functions are called by name, SSA values are passed by order to the function , the bool represents weather the return value from the function is HEAP allocated, CTLA will take ownership of any returned values,
    CallLocalFunction(ValueId, Box<String>, Vec<SSAValue>, bool),
    ///call externally defined function, same rules as above final type is just return type
    CallExternFunction(ValueId, Box<String>, Vec<SSAValue>, bool, TirType),
    ///creates a new struct interface, with a ValueId and with the specified values (YOU ARE RESPONSIBLE FOR KEEPING TRACK OF Field -> Idx mapping),
    ///second string is name, note please make the Tir AType a struct otherwise you will get a weird error
    CreateStructInterface(ValueId, Box<String>, TirType),
    ///takes the type (struct) and the EXACT SAME number of values which are then type checked and populated
    CreateStructLiteral(ValueId, TirType, Vec<SSAValue>),
    ///first SSAValue is the struct, second one is the field
    ReadStructLiteral(ValueId, SSAValue, u64),
    ///first SSA is struct, second is field, third is new val
    WriteStructLiteral(ValueId, SSAValue, u64, SSAValue),
    ///negates the given ssa value, assumes it is i1 but will do bitwise not otherwise
    Not(ValueId, SSAValue),
}
#[derive(PartialEq, Debug, Clone)]

pub struct Block {
    pub id: BlockId,
    pub ins: Vec<TIR>,
}
#[derive(PartialEq, Debug, Clone)]

pub struct Function {
    pub params: Vec<SSAValue>,
    pub body: Vec<Block>,
    pub name: Box<String>,
    pub ret_type: TirType,
}
pub struct TirBuilder {
    inst_counter: ValueId,
    block_counter: BlockId,
    pub funcs: Vec<Function>,
    curr_func: Option<usize>,  //index into self.funcs
    curr_block: Option<usize>, //index into self.curr_func.body,
}
impl TirBuilder {
    pub fn new() -> TirBuilder {
        return TirBuilder {
            inst_counter: 0,
            block_counter: 0,
            funcs: vec![],
            curr_func: None,
            curr_block: None,
        };
    }
    fn _next_value_id(&mut self) -> ValueId {
        self.inst_counter += 1;
        return self.inst_counter - 1;
    }
    fn _next_block_id(&mut self) -> BlockId {
        self.block_counter += 1;
        return self.block_counter - 1;
    }
    pub fn new_func(&mut self, name: Box<String>, params: Vec<SSAValue>, ret_type: TypeTok) {
        let func = Function {
            name: name,
            params: params,
            body: vec![],
            ret_type: self._type_tok_to_tir_type(ret_type),
        };
        let block = Block {
            id: self._next_block_id(),
            ins: vec![],
        };
        self.funcs.push(func);
        self.curr_func = Some(self.funcs.len() - 1); //item just pushed;
        self.funcs[self.curr_func.unwrap()].body.push(block);
        self.curr_block = Some(self.funcs[self.curr_func.unwrap()].body.len() - 1);
    }
    fn _type_tok_to_tir_type(&self, t: TypeTok) -> TirType {
        return match t {
            TypeTok::Int => TirType::I64,
            TypeTok::Bool => TirType::I1,
            TypeTok::Float => TirType::F64,
            TypeTok::Void => TirType::Void,
            _ => todo!("Chase, you have not implemented this yt"),
        };
    }
    pub fn iconst(&mut self, value: i64, val_type: TypeTok) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let t = self._type_tok_to_tir_type(val_type);
        let ins = TIR::IConst(id, value, t.clone());
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(t),
        });
    }
    pub fn numeric_infix(
        &mut self,
        left: SSAValue,
        right: SSAValue,
        op: InfixOp,
    ) -> Result<SSAValue, ToyError> {
        if let Some(left_t) = left.clone().ty
            && let Some(right_t) = right.clone().ty
        {
            if (left_t == TirType::I64 || left_t == TirType::F64)
                && (right_t == TirType::I64 || right_t == TirType::F64)
            {
                let n_op = match op {
                    InfixOp::Plus => NumericInfixOp::Plus,
                    InfixOp::Minus => NumericInfixOp::Minus,
                    InfixOp::Multiply => NumericInfixOp::Multiply,
                    InfixOp::Divide => NumericInfixOp::Divide,
                    InfixOp::Modulo => NumericInfixOp::Modulo,
                    _ => return Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType)),
                };
                let id = self._next_value_id();
                let ins = TIR::NumericInfix(id, left, right, n_op);
                self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
                    .ins
                    .push(ins);
                return Ok(SSAValue {
                    val: id,
                    ty: Some(if left_t == TirType::F64 || right_t == TirType::F64 {
                        TirType::F64
                    } else {
                        TirType::I64
                    }),
                });
            }
        }
        return Err(ToyError::new(ToyErrorType::ExpressionNotNumeric));
    }
    pub fn boolean_infix(
        &mut self,
        left: SSAValue,
        right: SSAValue,
        op: InfixOp,
    ) -> Result<SSAValue, ToyError> {
        if let Some(left_t) = left.clone().ty
            && let Some(right_t) = right.clone().ty
        {
            if left_t != TirType::I1 || right_t != TirType::I1 {
                return Err(ToyError::new(ToyErrorType::ExpressionNotBoolean));
            }
            let n_op = match op {
                InfixOp::And => BoolInfixOp::And,
                InfixOp::Or => BoolInfixOp::Or,
                InfixOp::Equals => BoolInfixOp::Equals,
                InfixOp::GreaterThan => BoolInfixOp::GreaterThan,
                InfixOp::GreaterThanEqt => BoolInfixOp::GreaterThanEqt,
                InfixOp::LessThan => BoolInfixOp::LessThan,
                InfixOp::LessThanEqt => BoolInfixOp::LessThenEqt,
                InfixOp::NotEquals => BoolInfixOp::NotEquals,
                _ => return Err(ToyError::new(ToyErrorType::InvalidOperationOnGivenType)),
            };
            let id = self._next_value_id();
            let ins = TIR::BoolInfix(id, left, right, n_op);
            self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
                .ins
                .push(ins);
            return Ok(SSAValue {
                val: id,
                ty: Some(TirType::I1),
            });
        } else {
            return Err(ToyError::new(ToyErrorType::ExpressionNotBoolean));
        }
    }
    ///first block returned is true block, second block is false block
    pub fn jump_cond(&mut self, cond: SSAValue) -> Result<(BlockId, BlockId), ToyError> {
        let true_id = self._next_block_id();
        let false_id = self._next_block_id();
        let true_block = Block {
            id: true_id,
            ins: vec![],
        };
        let false_block = Block {
            id: false_id,
            ins: vec![],
        };
        self.funcs[self.curr_func.unwrap()].body.push(true_block);
        self.funcs[self.curr_func.unwrap()].body.push(false_block);
        let ins = TIR::JumpCond(cond, true_id, false_id);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok((true_id, false_id));
    }
    pub fn jump(&mut self, val: SSAValue) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::Jump(id);
        self.funcs[self.curr_block.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        let val = SSAValue { val: id, ty: None };
        return Ok(val);
    }
    pub fn switch_block(&mut self, id: BlockId) {
        self.curr_block = Some(id);
    }
    ///switches the function to the given name, will set the block two the first block in the function, will crash if function is empty
    pub fn switch_fn(&mut self, name: String) -> Result<(), ToyError> {
        for i in 0..self.funcs.len() {
            if *self.funcs[i].name == name {
                self.curr_func = Some(i);
                self.curr_block = Some(0usize);
                return Ok(());
            }
        }
        return Err(ToyError::new(ToyErrorType::UndefinedFunction));
    }
    //I am leaving the Result because I am sure there will be some error later and I would rather not break the API
    pub fn ret(&mut self, val: SSAValue) -> Result<SSAValue, ToyError> {
        let id = self._next_block_id();
        let ins = TIR::Ret(val);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: None, //A function call is an expression, a return "statement" is not
        });
    }
    ///name is self evident, params are the values, passed by order, is_extern means if it is an externally defined function, is allocator means if the result from a function is heap allocated and CTLA will take ownership
    ///if the function is extern you must specify the return type, otherwise just put None, (or anything else the value is irrelevant, the return type will be looked up)
    pub fn call_func(
        &mut self,
        name: String,
        params: Vec<SSAValue>,
        is_extern: bool,
        is_allocator: bool,
        i_ret_type: Option<TirType>,
    ) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ret_type: TirType;
        let ins = if is_extern {
            ret_type = i_ret_type.clone().unwrap();
            TIR::CallExternFunction(
                id,
                Box::new(name),
                params,
                is_allocator,
                i_ret_type.unwrap(),
            )
        } else {
            ret_type = self
                .funcs
                .iter()
                .find(|f| *f.name == name)
                .map(|f| f.ret_type.clone())
                .unwrap();
            TIR::CallLocalFunction(id, Box::new(name), params, is_allocator)
        };
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(ret_type),
        });
    }
    pub fn create_struct_interface(&mut self, name: String, types: Vec<TirType>) -> TirType {
        let id = self._next_value_id();
        let struct_type = TirType::StructInterface(types);
        let ins = TIR::CreateStructInterface(id, Box::new(name), struct_type.clone());
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return struct_type;
    }
    pub fn create_struct_literal(
        &mut self,
        vals: Vec<SSAValue>,
        ty: TirType, //must be a struct
    ) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::CreateStructLiteral(id, ty.clone(), vals);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(ty),
        });
    }
    pub fn read_struct_literal(
        &mut self,
        struct_val: SSAValue,
        field_idx: u64,
        field_type: TirType, //type of the value at the field so on Point{x: float, y: float}. origin.x ret_type would be float (F64)
    ) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::ReadStructLiteral(id, struct_val, field_idx);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(field_type),
        });
    }
    pub fn write_struct_literal(
        &mut self,
        struct_val: SSAValue,
        field_idx: u64,
        new_val: SSAValue,
        field_type: TirType,
    ) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::WriteStructLiteral(id, struct_val, field_idx, new_val);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(field_type),
        });
    }

    pub fn not(&mut self, v: SSAValue) -> Result<SSAValue, ToyError> {
        if let Some(t) = v.clone().ty {
            return match t {
                TirType::I1 => {
                    let id = self._next_value_id();
                    let ins = TIR::Not(id, v);
                    self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
                        .ins
                        .push(ins);
                    return Ok(SSAValue {
                        val: id,
                        ty: Some(TirType::I1),
                    });
                }
                _ => Err(ToyError::new(ToyErrorType::ExpressionNotBoolean)),
            };
        }
        //this should be unreachable
        return Err(ToyError::new(ToyErrorType::ExpressionNotBoolean));
    }
}
