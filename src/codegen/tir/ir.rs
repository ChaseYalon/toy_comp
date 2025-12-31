#![allow(unused)]

use std::collections::HashMap;
type AllocationId = u64;
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
#[derive(PartialEq, Debug, Clone, Hash, Eq)]
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
    ///represents a pointer
    I8PTR,
}
impl TirType {
    pub fn to_string(&self) -> String {
        let s = match self {
            TirType::I64 => "i64".to_string(),
            TirType::I1 => "i1".to_string(),
            TirType::F64 => "f64".to_string(),
            TirType::StructInterface(types) => {
                let mut s = "struct".to_string();
                for t in types {
                    s.push_str(&t.to_string());
                }
                s
            }
            TirType::Void => "void".to_string(),
            TirType::I8PTR => "i8ptr".to_string(),
        };
        return s;
    }
}
#[derive(PartialEq, Debug, Clone, Hash, Eq)]

pub struct SSAValue {
    pub val: ValueId,
    pub ty: Option<TirType>,
}
#[derive(Debug, PartialEq, Clone)]
pub enum TIR {
    ///value as i64, regardless of weather it is an i64 or i1, and TirType to specify that
    IConst(ValueId, i64, TirType),
    ///value as f64 (if you are using f32, dont), tirType is ony used for later validation
    FConst(ValueId, f64, TirType),
    ///takes a specified integer ssa value (i64, plz dont do I1) and converts it to a float, final type should be F64
    ItoF(ValueId, SSAValue, TirType),
    ///numeric infix is any expression that returns a number so 5 + 3, NOT 5 < 3 (see: TBoolInfix)
    ///first valueId is handle for the expression, second and third are left and right, finally InfixOp
    NumericInfix(ValueId, SSAValue, SSAValue, NumericInfixOp),
    ///boolean infix is an expression that "returns" a boolean, can take numeric inputs, so both 5 + 3 and true || false are valid
    ///it is up to the "compiler" to ensure that 5 && 3 is either valid or never happens
    /// first value is the handle for the expression, second and third are left and right, 4th is operator
    BoolInfix(ValueId, SSAValue, SSAValue, BoolInfixOp),
    ///jumps to first block if the ValueId points to an i1 of "1" (true), second "0" (false), compiler will return an error if it is not an i1
    JumpCond(ValueId, SSAValue, BlockId, BlockId),
    ///jumps unconditionally to the BlockId specified
    JumpBlockUnCond(ValueId, BlockId),
    ///returns the value pointed to, as value if less then the word size, otherwise as a pointer
    Ret(ValueId, SSAValue),
    ///call locally defined function, functions are called by name, SSA values are passed by order to the function , the bool represents weather the return value from the function is HEAP allocated, CTLA will take ownership of any returned values,
    CallLocalFunction(ValueId, Box<String>, Vec<SSAValue>, bool, TirType),
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
    ///phi node where a given value corresponds to a given entrance block
    Phi(ValueId, Vec<BlockId>, Vec<SSAValue>),
    ///takes a string and puts it into global data
    GlobalString(ValueId, Box<String>),
}

impl TIR {
    pub fn get_id(&self) -> ValueId {
        match self {
            TIR::IConst(id, _, _) => *id,
            TIR::FConst(id, _, _) => *id,
            TIR::ItoF(id, _, _) => *id,
            TIR::NumericInfix(id, _, _, _) => *id,
            TIR::BoolInfix(id, _, _, _) => *id,
            TIR::JumpCond(id, _, _, _) => *id,
            TIR::JumpBlockUnCond(id, _) => *id,
            TIR::Ret(id, _) => *id,
            TIR::CallLocalFunction(id, _, _, _, _) => *id,
            TIR::CallExternFunction(id, _, _, _, _) => *id,
            TIR::CreateStructInterface(id, _, _) => *id,
            TIR::CreateStructLiteral(id, _, _) => *id,
            TIR::ReadStructLiteral(id, _, _) => *id,
            TIR::WriteStructLiteral(id, _, _, _) => *id,
            TIR::Not(id, _) => *id,
            TIR::Phi(id, _, _) => *id,
            TIR::GlobalString(id, _) => *id,
        }
    }
}
#[derive(PartialEq, Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub ins: Vec<TIR>,
}
#[derive(PartialEq, Clone, Debug)]
pub struct HeapAllocation {
    pub block: BlockId,
    ///functions are detected by name
    pub function: Box<String>,
    ///instruction that creates the heap allocation
    pub alloc_ins: SSAValue,
    ///unique id
    pub allocation_id: AllocationId,
    ///a vec representing every ssa value that references the allocation
    ///The string is the function name where the ref occurs, BlockId is self explanatory, ValueId is the id of the instruction where the ref occurs
    pub refs: Vec<(Box<String>, BlockId, ValueId)>,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Function {
    ///the ssa values where the params can be referenced
    pub params: Vec<SSAValue>,
    pub body: Vec<Block>,
    pub name: Box<String>,
    pub ret_type: TirType,
    pub ins_counter: usize,
    pub heap_counter: AllocationId,
    ///this tracks all the heap allocations in a given function, but the same heap allocation can be in multiple functions, if
    pub heap_allocations: Vec<HeapAllocation>,
}
#[derive(Clone)]
pub struct TirBuilder {
    block_counter: BlockId,
    pub funcs: Vec<Function>,
    curr_func: Option<usize>,                       //index into self.funcs
    curr_block: Option<usize>,                      //index into self.curr_func.body,
    extern_funcs: HashMap<String, (bool, TirType)>, //external function name to is_allocator, return_type
}
impl TirBuilder {
    pub fn new() -> TirBuilder {
        return TirBuilder {
            block_counter: 0,
            funcs: vec![],
            curr_func: None,
            curr_block: None,
            extern_funcs: HashMap::new(),
        };
    }
    fn _next_value_id(&mut self) -> ValueId {
        self.funcs[self.curr_func.unwrap()].ins_counter += 1;
        return self.funcs[self.curr_func.unwrap()].ins_counter - 1;
    }
    fn _next_alloc_id(&mut self) -> AllocationId {
        self.funcs[self.curr_func.unwrap()].heap_counter += 1;
        return self.funcs[self.curr_func.unwrap()].heap_counter - 1;
    }
    fn _next_block_id(&mut self) -> BlockId {
        self.block_counter += 1;
        return self.block_counter - 1;
    }
    pub fn new_func(&mut self, name: Box<String>, params: Vec<SSAValue>, ret_type: TypeTok) {
        let func = Function {
            name: name,
            params: params.clone(),
            body: vec![],
            ret_type: self.type_tok_to_tir_type(ret_type),
            ins_counter: params.len(),
            heap_allocations: vec![],
            heap_counter: 0,
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

    /// Updates the parameters of the current function
    pub fn set_func_params(&mut self, params: Vec<SSAValue>) {
        if let Some(func_idx) = self.curr_func {
            self.funcs[func_idx].params = params;
        }
    }

    pub fn iconst(&mut self, value: i64, val_type: TypeTok) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let t = self.type_tok_to_tir_type(val_type);
        let ins = TIR::IConst(id, value, t.clone());
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(t),
        });
    }
    pub fn fconst(&mut self, value: f64) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::FConst(id, value, TirType::F64);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(TirType::F64),
        });
    }
    pub fn i_to_f(&mut self, val: SSAValue) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::ItoF(id, val, TirType::F64);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: Some(TirType::F64),
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
                    _ => unreachable!(), // parser validated
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
        unreachable!(); // parser validated
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
            // Comparison operators (>, <, >=, <=, ==, !=) can work on I64 values
            // Logical operators (&& ||) require I1 values
            let is_comparison = matches!(
                op,
                InfixOp::LessThan
                    | InfixOp::GreaterThan
                    | InfixOp::GreaterThanEqt
                    | InfixOp::LessThanEqt
                    | InfixOp::Equals
                    | InfixOp::NotEquals
            );
            let is_logical = matches!(op, InfixOp::And | InfixOp::Or);

            if is_comparison {
                // Comparison operators: both operands must be the same type (I64)
                if left_t != right_t || left_t != TirType::I64 {
                    unreachable!(); // parser validated
                }
            } else if is_logical {
                // Logical operators: both operands must be I1
                if left_t != TirType::I1 || right_t != TirType::I1 {
                    unreachable!(); // parser validated
                }
            } else {
                unreachable!(); // parser validated
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
                _ => unreachable!(), // parser validated
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
            unreachable!(); // parser validated
        }
    }
    ///first block returned is true block, second block is false block, does not switch blocks
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
        let id = self._next_value_id();
        let ins = TIR::JumpCond(id, cond, true_id, false_id);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok((true_id, false_id));
    }
    pub fn switch_block(&mut self, id: BlockId) {
        // Find the block by its ID and get its index in the body vector
        if let Some(func_idx) = self.curr_func {
            if let Some(block_idx) = self.funcs[func_idx].body.iter().position(|b| b.id == id) {
                self.curr_block = Some(block_idx);
            }
        }
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
        unreachable!(); // parser validated
    }
    //I am leaving the Result because I am sure there will be some error later and I would rather not break the API
    pub fn ret(&mut self, val: SSAValue) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::Ret(id, val.clone());
        let curr_func_name = self.funcs[self.curr_func.unwrap()].name.clone();
        let curr_block = self.curr_block.unwrap();
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    if alloc.alloc_ins == val {
                        alloc.refs.push((curr_func_name.clone(), curr_block, id));
                    }
                })
        });
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue {
            val: id,
            ty: None, //A function call is an expression, a return "statement" is not
        });
    }
    /// Calls a locally defined function by name.
    /// The return type is automatically looked up from the function definition.
    /// `is_allocator` indicates if the return value is heap-allocated (CTLA takes ownership).
    pub fn call_local(
        &mut self,
        name: String,
        params: Vec<SSAValue>,
        is_allocator: bool,
    ) -> Result<SSAValue, ToyError> {
        let curr_func_name = self.funcs[self.curr_func.unwrap()].name.clone();
        let curr_block = self.curr_block.unwrap();
        let id = self._next_value_id();
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    params.iter().for_each(|p| {
                        if p == &alloc.alloc_ins {
                            alloc.refs.push((curr_func_name.clone(), curr_block, p.val));
                        }
                    })
                })
        });
        let ret_type = self
            .funcs
            .iter()
            .find(|f| *f.name == name)
            .map(|f| f.ret_type.clone())
            .unwrap(); // parser validated

        let ins = TIR::CallLocalFunction(
            id,
            Box::new(name),
            params.clone(),
            is_allocator,
            ret_type.clone(),
        );
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);

        let ret_ins = SSAValue {
            val: id,
            ty: Some(ret_type),
        };
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    if ret_ins == alloc.alloc_ins {
                        alloc.refs.push((curr_func_name.clone(), curr_block, id));
                    }
                })
        });
        return Ok(ret_ins);
    }

    /// Calls an externally defined function by name.
    /// The function must be registered with `register_extern` first.
    /// Returns an error if the function is not registered.
    pub fn call_extern(
        &mut self,
        name: String,
        params: Vec<SSAValue>,
    ) -> Result<SSAValue, ToyError> {
        let (is_allocator, ret_type) = self.extern_funcs.get(&name).cloned().unwrap(); // parser validated
        let curr_func_name = self.funcs[self.curr_func.unwrap()].name.clone();
        let curr_block = self.curr_block.unwrap();
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    params.iter().for_each(|p| {
                        if p == &alloc.alloc_ins {
                            alloc.refs.push((curr_func_name.clone(), curr_block, p.val));
                        }
                    })
                })
        });
        let id = self._next_value_id();
        let ins =
            TIR::CallExternFunction(id, Box::new(name), params, is_allocator, ret_type.clone());
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins.clone()); //SAFETY: that is the real value of the ins and the only ref later is for the ins_value in the HeapAllocation
        let val = SSAValue {
            val: id,
            ty: Some(ret_type),
        };
        if is_allocator {
            let curr_block_id =
                self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()].id;
            let alloc = HeapAllocation {
                block: curr_block_id,
                allocation_id: self._next_alloc_id(),
                function: self.funcs[self.curr_func.unwrap()].name.clone(),
                refs: vec![],
                alloc_ins: val.clone(),
            };
            self.funcs[self.curr_func.unwrap()]
                .heap_allocations
                .push(alloc)
        }
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    if val == alloc.alloc_ins {
                        alloc
                            .refs
                            .push((curr_func_name.clone(), curr_block, val.val));
                    }
                })
        });
        return Ok(val);
    }

    /// Calls a function by name, automatically determining if it's local or extern.
    /// Checks local functions first, then falls back to registered extern functions.
    pub fn call(&mut self, name: String, params: Vec<SSAValue>) -> Result<SSAValue, ToyError> {
        // Check if it's a local function first
        if self.funcs.iter().any(|f| *f.name == name) {
            return self.call_local(name, params, false);
        }

        // Otherwise, try extern
        if self.extern_funcs.contains_key(&name) {
            return self.call_extern(name, params);
        }

        unreachable!(); // parser validated
    }
    //I dont like ths, it should be in the call_extern
    /// Calls an externally defined function that returns void.
    /// Use this for functions like toy_write_to_arr that don't return a value.
    pub fn call_extern_void(
        &mut self,
        name: String,
        params: Vec<SSAValue>,
    ) -> Result<(), ToyError> {
        //params should never be freed, so no need to track refs
        let (is_allocator, ret_type) = self.extern_funcs.get(&name).cloned().unwrap(); // parser validated

        let id = self._next_value_id();
        let ins = TIR::CallExternFunction(id, Box::new(name), params, is_allocator, ret_type);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);

        Ok(())
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
        let ins = TIR::CreateStructLiteral(id, ty.clone(), vals.clone());
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        let ret_ins = SSAValue {
            val: id,
            ty: Some(ty),
        };
        let curr_func_name = self.funcs[self.curr_func.unwrap()].name.clone();
        let curr_block = self.curr_block.unwrap();
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    vals.iter().for_each(|v| {
                        if v == &alloc.alloc_ins {
                            alloc.refs.push((curr_func_name.clone(), curr_block, v.val));
                        }
                    });
                    if alloc.alloc_ins == ret_ins {
                        alloc.refs.push((curr_func_name.clone(), curr_block, id));
                    }
                })
        });
        return Ok(ret_ins);
    }
    pub fn read_struct_literal(
        &mut self,
        struct_val: SSAValue,
        field_idx: u64,
        field_type: TirType, //type of the value at the field so on Point{x: float, y: float}. origin.x ret_type would be float (F64)
    ) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let struct_val_clone = struct_val.clone();
        let curr_func_name = self.funcs[self.curr_func.unwrap()].name.clone();
        let curr_block = self.curr_block.unwrap();
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    if alloc.alloc_ins == struct_val_clone {
                        alloc
                            .refs
                            .push((curr_func_name.clone(), curr_block, struct_val_clone.val));
                    }
                })
        });
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
        let struct_val_clone = struct_val.clone();
        let new_val_clone = new_val.clone();
        let curr_func_name = self.funcs[self.curr_func.unwrap()].name.clone();
        let curr_block = self.curr_block.unwrap();
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    if alloc.alloc_ins == struct_val_clone {
                        alloc
                            .refs
                            .push((curr_func_name.clone(), curr_block, struct_val_clone.val));
                    }
                    if alloc.alloc_ins == new_val_clone {
                        alloc
                            .refs
                            .push((curr_func_name.clone(), curr_block, new_val_clone.val));
                    }
                })
        });
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
                _ => unreachable!(), // parser validated
            };
        }
        //this should be unreachable
        unreachable!(); // parser validated
    }
    pub fn jump_block_un_cond(&mut self, block_id: BlockId) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::JumpBlockUnCond(id, block_id);
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        return Ok(SSAValue { val: id, ty: None });
    }
    /// Emit a phi node that takes values from multiple predecessor blocks
    /// block_ids: the IDs of the predecessor blocks
    /// values: the SSA values from each predecessor block (must match order of block_ids)
    pub fn emit_phi(
        &mut self,
        block_ids: Vec<BlockId>,
        values: Vec<SSAValue>,
    ) -> Result<SSAValue, ToyError> {
        if block_ids.is_empty() || values.is_empty() || block_ids.len() != values.len() {
            unreachable!(); // parser validated
        }
        let id = self._next_value_id();
        let ins = TIR::Phi(id, block_ids, values.clone());
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        let ret_val = SSAValue {
            val: id,
            ty: values[0].ty.clone(),
        };
        let curr_func_name = self.funcs[self.curr_func.unwrap()].name.clone();
        let curr_block = self.curr_block.unwrap();
        self.funcs.iter_mut().for_each(|f| {
            f.heap_allocations
                .iter_mut()
                .for_each(|alloc: &mut HeapAllocation| {
                    let mut matched = false;
                    values.iter().for_each(|v| {
                        if v == &alloc.alloc_ins {
                            alloc.refs.push((curr_func_name.clone(), curr_block, v.val));
                            matched = true;
                        }
                    });
                    if matched {
                        alloc.refs.push((curr_func_name.clone(), curr_block, id));
                    }
                })
        });
        // Use the type from the first value
        return Ok(ret_val);
    }
    pub fn create_block(&mut self) -> Result<BlockId, ToyError> {
        let id = self._next_block_id();
        let b = Block {
            id: id,
            ins: vec![],
        };
        self.funcs[self.curr_func.unwrap()].body.push(b);
        return Ok(id);
    }
    /// Insert a TIR instruction at the beginning of a block (useful for phi nodes)
    pub fn insert_at_block_start(&mut self, block_id: BlockId, ins: TIR) -> Result<(), ToyError> {
        if let Some(func_idx) = self.curr_func {
            if let Some(block_idx) = self.funcs[func_idx]
                .body
                .iter()
                .position(|b| b.id == block_id)
            {
                self.funcs[func_idx].body[block_idx].ins.insert(0, ins);
                return Ok(());
            }
        }
        unreachable!(); // parser validated
    }
    /// Get the number of instructions currently in a block
    pub fn get_block_ins_count(&self, block_id: BlockId) -> Result<usize, ToyError> {
        if let Some(func_idx) = self.curr_func {
            if let Some(block) = self.funcs[func_idx].body.iter().find(|b| b.id == block_id) {
                return Ok(block.ins.len());
            }
        }
        unreachable!(); // parser validated
    }
    /// Get the next value ID without emitting an instruction
    pub fn peek_next_value_id(&self) -> ValueId {
        self.funcs[self.curr_func.unwrap()].ins_counter
    }
    /// Manually allocate a new value ID
    pub fn alloc_value_id(&mut self) -> ValueId {
        self._next_value_id()
    }
    //utils
    pub fn generic_ssa(&mut self, t: TypeTok) -> SSAValue {
        let id = self._next_value_id();
        let ty = self.type_tok_to_tir_type(t);
        return SSAValue {
            val: id,
            ty: Some(ty),
        };
    }
    pub fn get_func_ret_type(&self, name: String) -> Result<TirType, ToyError> {
        Ok(self
            .funcs
            .iter()
            .find(|f| *f.name == name)
            .map(|f| f.clone().ret_type)
            .unwrap()) // parser validated
    }
    pub fn register_extern(&mut self, name: String, is_allocator: bool, ret_type: TypeTok) {
        self.extern_funcs
            .insert(name, (is_allocator, self.type_tok_to_tir_type(ret_type)));
    }
    pub fn global_string(&mut self, name: String) -> Result<SSAValue, ToyError> {
        let id = self._next_value_id();
        let ins = TIR::GlobalString(id, Box::new(name));
        self.funcs[self.curr_func.unwrap()].body[self.curr_block.unwrap()]
            .ins
            .push(ins);
        let val = SSAValue {
            val: id,
            ty: Some(TirType::I8PTR),
        };
        let mut val2 = self.call_extern("toy_malloc".to_string(), vec![val])?;
        val2.ty = Some(TirType::I8PTR);
        //we need to update the type of the allocation instruction in the heap allocation list
        //otherwise the CTLA will not be able to find the allocation
        self.funcs[self.curr_func.unwrap()]
            .heap_allocations
            .iter_mut()
            .for_each(|alloc| {
                if alloc.alloc_ins.val == val2.val {
                    alloc.alloc_ins.ty = Some(TirType::I8PTR);
                }
            });
        return Ok(val2);
    }
    pub fn inject_type_param(
        &mut self,
        t: &TypeTok,
        inject_dimension: bool,
        param_values: &mut Vec<SSAValue>,
    ) -> Result<(), ToyError> {
        let (n, degree) = match t {
            &TypeTok::Str => (0, 0),
            &TypeTok::Bool => (1, 0),
            &TypeTok::Int => (2, 0),
            &TypeTok::Float => (3, 0),
            &TypeTok::StrArr(n) => (4, n),
            &TypeTok::BoolArr(n) => (5, n),
            &TypeTok::IntArr(n) => (6, n),
            &TypeTok::FloatArr(n) => (7, n),
            &TypeTok::StructArr(_, n) => (8, n), // struct arrays use type code 8, must have str method to be printed
            _ => unreachable!(),                 // parser validated
        };
        let v = self.iconst(n, TypeTok::Int)?;
        param_values.push(v);
        if inject_dimension {
            let d = self.iconst(degree as i64, TypeTok::Int)?;
            param_values.push(d);
        }
        return Ok(());
    }
    pub fn type_tok_to_tir_type(&self, t: TypeTok) -> TirType {
        return match t {
            TypeTok::Int => TirType::I64,
            TypeTok::Bool => TirType::I1,
            TypeTok::Float => TirType::F64,
            TypeTok::Void => TirType::Void,
            TypeTok::Str
            | TypeTok::StrArr(_)
            | TypeTok::BoolArr(_)
            | TypeTok::IntArr(_)
            | TypeTok::FloatArr(_)
            | TypeTok::AnyArr(_) => TirType::I8PTR,
            TypeTok::Struct(i) => {
                let mut types: Vec<TirType> = vec![];
                for (_, ty) in i.iter() {
                    types.push(self.type_tok_to_tir_type(*ty.to_owned()));
                }
                TirType::StructInterface(types)
            }
            _ => todo!("Chase you have not done the TypeTok -> TirType conversion for {t:?}"),
        };
    }
    ///will erase all functions saved in the builder and set current func to the indicated
    pub fn set_funcs(&mut self, funcs: Vec<Function>, func: usize) {
        self.funcs = funcs;
        self.curr_func = Some(func);
        return self.curr_block = Some(0);
    }
    pub fn detect_heap_allocations(&self) -> Vec<HeapAllocation> {
        let mut allocs: Vec<HeapAllocation> = vec![];
        for func in self.funcs.clone() {
            allocs.extend(func.heap_allocations);
        }
        return allocs;
    }
    ///Will add a manually created TIR instruction before the specified instruction
    ///Will NOT move the cursor
    pub fn splice_free_before(
        &mut self,
        func_name: String,
        block_id: BlockId,
        before_ins: usize,
        to_free_val: SSAValue,
        free_func_name: String,
    ) {
        let ins = TIR::CallExternFunction(
            self._next_value_id(),
            Box::new(free_func_name),
            vec![to_free_val],
            false,
            TirType::Void,
        );

        let func = self
            .funcs
            .iter_mut()
            .find(|f| *f.name == func_name)
            .unwrap();
        let block = func.body.iter_mut().find(|b| b.id == block_id).unwrap();
        block.ins.insert(before_ins, ins);
    }
}
