use super::tir::ir::BlockId;
use crate::{
    codegen::{
        SSAValue,
        ctla::aliasing::AliasAndEncapsulationTracker,
        tir::ir::{Block, Function, HeapAllocation, TIR, TirBuilder, TirType, ValueId},
    },
    driver::Driver,
    errors::ToyError,
};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::{cell::RefCell, io::Write};
pub mod aliasing;
pub mod cfg;
use cfg::{CFGBlock, CFGFunction, EscapeType};
use serde::{Deserialize, Serialize};
use std::fs;
pub struct CTLA {
    builder: Rc<RefCell<TirBuilder>>,
    cfg_functions: Vec<CFGFunction>,
    alias_detector: AliasAndEncapsulationTracker,
    original_text: Option<String>,
    /// function name -> (phi result value id -> (predecessor block, operand value id) list).
    /// Cached so phi-chain walks are O(1) lookups instead of scanning all instructions per step.
    phi_operands_by_func: HashMap<String, HashMap<ValueId, Vec<(BlockId, ValueId)>>>,
    /// function name -> set of blocks containing a non-returning panic call.
    panic_blocks_by_func: HashMap<String, HashSet<BlockId>>,
    stats: Option<CTLAStats>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OwnedField {
    pub index: usize,
    pub is_array: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionSummary {
    pub name: String,
    pub aliased_parameters: Vec<usize>,
    pub encapsulated_parameters: Vec<usize>,
    pub escaped_parameters: Vec<usize>,
    #[serde(default)]
    pub return_owned_fields: Vec<OwnedField>,
    /// (arr_param_idx, elem_param_idx): this function stores param[elem] into param[arr]
    #[serde(default)]
    pub param_encapsulates_pairs: Vec<(usize, usize)>,
}
impl FunctionSummary {
    pub fn new(
        name: String,
        aliased_parameters: Vec<usize>,
        encapsulated_parameters: Vec<usize>,
        escaped_parameters: Vec<usize>,
        return_owned_fields: Vec<OwnedField>,
        param_encapsulates_pairs: Vec<(usize, usize)>,
    ) -> FunctionSummary {
        return FunctionSummary {
            name: name,
            aliased_parameters,
            encapsulated_parameters,
            escaped_parameters,
            return_owned_fields,
            param_encapsulates_pairs,
        };
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CTLAStats {
    // Compile-time fields populated by CTLA
    pub alloc_count: u64,
    pub alias_count: u64,
    pub encap_count: u64,
    /// Fraction (0.0–1.0) of allocations that escape their creating function
    pub escape_func_pct: f64,
    /// Fraction (0.0–1.0) of allocations that escape their creating module
    pub escape_mod_pct: f64,
    /// Total fixed-point iterations across all alias propagation passes
    pub fp_iters: u64,
    // Runtime fields filled in by the runtime after execution
    pub escape_prog_pct: Option<f64>,
    pub total_bytes: Option<u64>,
    pub malloc_calls: Option<u64>,
    pub lifetime_mean_ns: Option<u64>,
    pub lifetime_median_ns: Option<u64>,
    pub lifetime_min_ns: Option<u64>,
    pub lifetime_max_ns: Option<u64>,
}

/// Current on-disk `.ctla` summary schema version. Bump whenever the meaning or shape of a
/// `FunctionSummary` field changes so stale blobs are ignored on load (see `Driver`). v3 split
/// `encapsulated_parameters` from `aliased_parameters` (element-reads vs whole-value aliases).
pub const CTLA_SCHEMA_VERSION: u64 = 3;
/// Name suffix for the borrowed clone of an encapsulating wrapper (see
/// `emit_borrowed_wrapper_variants` / `mark_wrapper_writes_borrowed`). Importers derive the clone
/// name from the original by appending this, so it must stay in sync on both sides.
pub const BORROWED_WRAPPER_SUFFIX: &str = "__borrowed";
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CTLASchema {
    pub schema_version: u64,
    pub summaries: Vec<FunctionSummary>,
    pub input_hash: String,
    pub module_name: String,
}
impl CTLASchema {
    pub fn new(
        schema_version: u64,
        summaries: Vec<FunctionSummary>,
        input_hash: String,
        module_name: String,
    ) -> CTLASchema {
        return CTLASchema {
            schema_version,
            summaries,
            input_hash,
            module_name,
        };
    }
}
//COMPILE TIME LIFETIME ANALYSIS
impl CTLA {
    pub fn new() -> CTLA {
        let b = Rc::new(RefCell::new(TirBuilder::new()));
        let alias_detector = AliasAndEncapsulationTracker::new(&b);

        CTLA {
            builder: b,
            cfg_functions: vec![],
            alias_detector,
            original_text: None,
            phi_operands_by_func: HashMap::new(),
            panic_blocks_by_func: HashMap::new(),
            stats: None,
        }
    }

    pub fn stats(&self) -> Option<&CTLAStats> {
        self.stats.as_ref()
    }

    /// Builds the per-function phi-operand and panic-block indexes from the current builder.
    /// Must be called after `self.builder` is populated and before any phi-chain walks.
    fn build_phi_index(&mut self) {
        let mut phi_operands_by_func: HashMap<String, HashMap<ValueId, Vec<(BlockId, ValueId)>>> =
            HashMap::new();
        let mut panic_blocks_by_func: HashMap<String, HashSet<BlockId>> = HashMap::new();

        let builder = self.builder.borrow();
        for f in &builder.funcs {
            let func_name = (*f.name).clone();
            let phi_map = phi_operands_by_func.entry(func_name.clone()).or_default();
            let panic_set = panic_blocks_by_func.entry(func_name).or_default();
            for block in &f.body {
                let mut block_has_panic = false;
                for ins in &block.ins {
                    match ins {
                        TIR::Phi(out_id, block_ids, vals) => {
                            let operands: Vec<(BlockId, ValueId)> = block_ids
                                .iter()
                                .zip(vals.iter())
                                .map(|(bid, v)| (*bid, v.val))
                                .collect();
                            phi_map.insert(*out_id, operands);
                        }
                        TIR::CallExternFunction(_, name, _, _, _, _)
                            if **name == *"std::sys::panic_str" =>
                        {
                            block_has_panic = true;
                        }
                        _ => {}
                    }
                }
                if block_has_panic {
                    panic_set.insert(block.id);
                }
            }
        }
        drop(builder);

        self.phi_operands_by_func = phi_operands_by_func;
        self.panic_blocks_by_func = panic_blocks_by_func;
    }

    pub fn set_external_modules(&mut self, modules: HashMap<String, Vec<FunctionSummary>>) {
        self.alias_detector.set_external_modules(modules);
    }

    pub fn set_original_text(&mut self, text: String) {
        self.original_text = Some(text);
    }
    pub fn cfg_functions(&self) -> &Vec<CFGFunction> {
        &self.cfg_functions
    }
    #[allow(unused)]
    pub fn alias_tracker(&self) -> &AliasAndEncapsulationTracker {
        &self.alias_detector
    }
    fn is_terminator_ins(&self, ins: &TIR) -> bool {
        return matches!(
            ins,
            TIR::Ret(_, _) | TIR::JumpCond(_, _, _, _) | TIR::JumpBlockUnCond(_, _)
        );
    }
    ///checks whether some SSA value could refer to a specific allocation, but only by following phi chains.
    fn value_may_be_allocation_via_phi(
        &self,
        func: &Function,
        value_id: ValueId,
        alloc: &HeapAllocation,
        visited: &mut HashSet<ValueId>,
    ) -> bool {
        let seeds = HashSet::from([alloc.alloc_ins.val]);
        self.value_may_match_seed_via_phi(func, value_id, &seeds, visited)
    }
    fn value_may_match_seed_via_phi(
        &self,
        func: &Function,
        value_id: ValueId,
        seeds: &HashSet<ValueId>,
        visited: &mut HashSet<ValueId>,
    ) -> bool {
        if seeds.contains(&value_id) {
            return true;
        }
        if visited.contains(&value_id) {
            return false;
        }
        visited.insert(value_id);

        let Some(func_phis) = self.phi_operands_by_func.get(func.name.as_str()) else {
            return false;
        };
        // A value id absent from the phi index is either not a phi or doesn't exist;
        // either way it cannot match a seed by following phi chains.
        let Some(operands) = func_phis.get(&value_id) else {
            return false;
        };

        let panic_blocks = self.panic_blocks_by_func.get(func.name.as_str());
        operands.iter().any(|(bid, v)| {
            if panic_blocks.is_some_and(|pb| pb.contains(bid)) {
                return false;
            }
            self.value_may_match_seed_via_phi(func, *v, seeds, visited)
        })
    }
    fn allocation_protected_values_in_function(
        &self,
        alloc: &HeapAllocation,
        function_name: &str,
    ) -> HashSet<ValueId> {
        let mut protected_ids: HashSet<ValueId> = HashSet::new();

        if alloc.function.as_ref() == function_name {
            protected_ids.insert(alloc.alloc_ins.val);
        }

        alloc
            .refs
            .iter()
            .filter(|(f, _, _)| f.as_ref() == function_name)
            .for_each(|(_, _, value_id)| {
                protected_ids.insert(*value_id);
            });

        alloc
            .aliases
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, _, value_id)| {
                protected_ids.insert(*value_id);
            });

        alloc
            .encapsulators
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, _, value_id)| {
                protected_ids.insert(*value_id);
            });

        protected_ids
    }
    fn allocation_tracked_blocks_in_function(
        &self,
        alloc: &HeapAllocation,
        function_name: &str,
    ) -> HashSet<BlockId> {
        let mut tracked_blocks: HashSet<BlockId> = HashSet::new();

        if alloc.function.as_ref() == function_name {
            tracked_blocks.insert(alloc.block);
        }

        alloc
            .refs
            .iter()
            .filter(|(f, _, _)| f.as_ref() == function_name)
            .for_each(|(_, b, _)| {
                tracked_blocks.insert(*b);
            });

        alloc
            .aliases
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, b, _)| {
                tracked_blocks.insert(*b);
            });

        alloc
            .encapsulators
            .iter()
            .filter(|(f, _, _)| f.as_str() == function_name)
            .for_each(|(_, b, _)| {
                tracked_blocks.insert(*b);
            });

        tracked_blocks
    }

    /// Finds the exact index in a given block where it is safe to insert a free call
    /// Assumes the block is the correct place for the free
    fn free_insertion_index_for_block(
        &self,
        func: &Function,
        block_id: BlockId,
        alloc: &HeapAllocation,
    ) -> usize {
        let block = func.body.iter().find(|b| b.id == block_id).unwrap();
        let protected_ids = self.allocation_protected_values_in_function(alloc, func.name.as_ref());

        let last_ref_idx = block
            .ins
            .iter()
            .enumerate()
            .filter(|(_, ins)| self.instruction_uses_any_value(ins, &protected_ids))
            .map(|(idx, _)| idx)
            .max();

        let terminator_idx = block
            .ins
            .last()
            .and_then(|ins| self.is_terminator_ins(ins).then_some(block.ins.len() - 1));

        let mut insertion_idx = match last_ref_idx {
            Some(idx) => idx + 1,
            None => terminator_idx.unwrap_or(block.ins.len()),
        };

        let has_same_func_encapsulator =
            alloc.encapsulators.iter().any(|(f, _, _)| *f == *func.name);
        // A struct allocation must outlive every value read out of its fields (a field read-out
        // aliases the field's heap content, which the owned-field deep-free reclaims). Those
        // read-out values can be used anywhere in the block (e.g. `println(p.s)`), so place the
        // struct free — and the owned-field frees spliced before it — at the block terminator.
        let is_struct_alloc = self.alloc_type_to_free_func(alloc) == "toy_free_struct";
        if has_same_func_encapsulator || is_struct_alloc {
            insertion_idx = terminator_idx.unwrap_or(block.ins.len());
        }

        // LLVM requires phi nodes to be grouped at the top of a basic block. If the
        // chosen insertion point lands inside the phi prologue, advance past it.
        let first_non_phi_idx = block
            .ins
            .iter()
            .position(|ins| !matches!(ins, TIR::Phi(_, _, _)))
            .unwrap_or(block.ins.len());
        if insertion_idx < first_non_phi_idx {
            insertion_idx = first_non_phi_idx;
        }

        if let Some(term_idx) = terminator_idx {
            if insertion_idx > term_idx {
                insertion_idx = term_idx;
            }
        }

        return insertion_idx;
    }
    fn block_returns_allocation_or_alias(
        &self,
        func: &Function,
        block_id: BlockId,
        alloc: &HeapAllocation,
    ) -> bool {
        let Some(block) = func.body.iter().find(|b| b.id == block_id) else {
            return false;
        };
        let Some(TIR::Ret(_, ret_val)) = block.ins.last() else {
            return false;
        };
        if ret_val.ty.is_none() {
            return false;
        }

        let protected_ids = self.allocation_protected_values_in_function(alloc, func.name.as_ref());
        if protected_ids.contains(&ret_val.val) {
            let ret_is_phi = func
                .body
                .iter()
                .flat_map(|b| b.ins.iter())
                .any(|ins| ins.get_id() == ret_val.val && matches!(ins, TIR::Phi(_, _, _)));
            if !ret_is_phi {
                return true;
            }
        }

        let mut visited = HashSet::new();
        self.value_may_be_allocation_via_phi(func, ret_val.val, alloc, &mut visited)
    }
    /// Determines if the given instruction references any of the value or any of its aliases
    fn instruction_uses_any_value(&self, ins: &TIR, aliases: &HashSet<ValueId>) -> bool {
        let uses = |value: &SSAValue| aliases.contains(&value.val);
        return match ins {
            TIR::ItoF(_, value, _) => uses(value),
            TIR::NumericInfix(_, left, right, _) => uses(left) || uses(right),
            TIR::BoolInfix(_, left, right, _) => uses(left) || uses(right),
            TIR::JumpCond(_, cond, _, _) => uses(cond),
            TIR::Ret(_, value) => uses(value),
            TIR::CallLocalFunction(_, _, params, _, _)
            | TIR::CallExternFunction(_, _, params, _, _, _)
            | TIR::CreateStructLiteral(_, _, params)
            | TIR::Phi(_, _, params) => params.iter().any(uses),
            TIR::CallFuncPtr(_, callable, params, _, _) => {
                uses(callable) || params.iter().any(uses)
            }
            TIR::ReadStructLiteral(_, struct_value, _) => uses(struct_value),
            TIR::WriteStructLiteral(_, struct_value, _, new_value) => {
                uses(struct_value) || uses(new_value)
            }
            TIR::Not(_, value) => uses(value),
            TIR::IConst(_, _, _)
            | TIR::FConst(_, _, _)
            | TIR::JumpBlockUnCond(_, _)
            | TIR::CreateStructInterface(_, _, _)
            | TIR::GlobalString(_, _)
            | TIR::FuncPtr(_, _) => false,
        };
    }

    /// The runtime deallocators CTLA itself inserts. Passing an allocation to one of these frees it;
    /// it is never an escape, so escape analysis must ignore these call sites (they may already be
    /// present when analysis re-runs, e.g. during stats computation).
    fn is_free_func_name(name: &str) -> bool {
        matches!(
            name,
            "toy_free"
                | "toy_free_arr"
                | "toy_deep_free_arr"
                | "toy_free_struct"
                | "toy_free_evicted"
                | "toy_deep_free_arr_evicted"
        )
    }

    /// True when this "allocation" is actually the result of a call that returns an alias of one of
    /// its arguments and owns no fields of its own (e.g. `read_rand` returning an array element). It
    /// points into existing memory, so it is not a fresh allocation: it must not be freed, and it
    /// does not escape anything.
    fn allocation_is_returned_param_alias(&self, alloc: &HeapAllocation) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == *alloc.function) else {
            return false;
        };
        let alloc_ins = func
            .body
            .iter()
            .flat_map(|b| b.ins.iter())
            .find(|ins| ins.get_id() == alloc.alloc_ins.val);
        let Some(
            TIR::CallLocalFunction(_, callee_name, params, _, _)
            | TIR::CallExternFunction(_, callee_name, params, _, _, _),
        ) = alloc_ins
        else {
            return false;
        };
        let summary = self
            .cfg_functions
            .iter()
            .find(|f| *f.func.name == **callee_name)
            .map(|f| (f.returns_alias_of_parameter.clone(), f.return_owned_fields.clone()))
            .or_else(|| {
                self.alias_detector
                    .get_external_summary(callee_name.as_ref())
                    .map(|s| (s.aliased_parameters.clone(), s.return_owned_fields.clone()))
            });
        match summary {
            Some((aliased_params, owned_fields)) => {
                owned_fields.is_empty()
                    && aliased_params.iter().any(|idx| params.get(*idx).is_some())
            }
            None => false,
        }
    }

    /// True when this allocation (or one of its same-function aliases) is stored into an array as
    /// the element argument of a `toy_arr_swap` / `toy_write_to_arr` call. Such values are owned by
    /// the array and reclaimed via the array's deep-free (survivors) or the swap-eviction free
    /// (overwritten elements) — never by a per-element free, which would double-free.
    fn allocation_written_into_array(&self, alloc: &HeapAllocation) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == *alloc.function) else {
            return false;
        };
        let mut value_ids: HashSet<ValueId> = HashSet::new();
        value_ids.insert(alloc.alloc_ins.val);
        for (f, _, v) in &alloc.aliases {
            if *f == *alloc.function {
                value_ids.insert(*v);
            }
        }
        for (f, _, v) in &alloc.refs {
            if **f == *alloc.function {
                value_ids.insert(*v);
            }
        }
        // Direct writes: the value is the elem arg of a toy_arr_swap / toy_write_to_arr in this
        // function (e.g. `arr[i] = x` or an array literal).
        let direct = func.body.iter().flat_map(|b| b.ins.iter()).any(|ins| {
            if let TIR::CallExternFunction(_, name, params, _, _, _) = ins {
                if name.as_ref() != "toy_arr_swap" && name.as_ref() != "toy_write_to_arr" {
                    return false;
                }
                if !params.get(1).is_some_and(|p| value_ids.contains(&p.val)) {
                    return false;
                }
                // Only str / nested-array elements are array-owned under the swap+deep-free model.
                // Bare struct elements (code 8) keep their per-element toy_free_struct.
                let code = params
                    .get(3)
                    .and_then(|p| self.resolve_iconst(&alloc.function, p.val));
                matches!(code, Some(0) | Some(4) | Some(5) | Some(6) | Some(7))
            } else {
                false
            }
        });
        if direct {
            return true;
        }
        // Wrapper writes: the value is encapsulated into a local array via a wrapper call
        // (e.g. fuzz.write_arr), captured by the encapsulator set rather than a direct call.
        // The array owns it, so suppress its per-element free.
        alloc.encapsulators.iter().any(|(f, _, enc_v)| {
            if f != alloc.function.as_ref() {
                return false;
            }
            // Skip self-encapsulation: reading an element out of an array and writing it back makes
            // the array appear encapsulated by itself — that must not suppress the array's own free.
            if *enc_v == alloc.alloc_ins.val || value_ids.contains(enc_v) {
                return false;
            }
            let Some(TIR::CallExternFunction(_, name, params, _, _, _)) = func
                .body
                .iter()
                .flat_map(|b| b.ins.iter())
                .find(|i| i.get_id() == *enc_v)
            else {
                return false;
            };
            if name.as_ref() != "toy_malloc_arr" {
                return false;
            }
            let code = params
                .get(1)
                .and_then(|p| self.resolve_iconst(&alloc.function, p.val));
            matches!(code, Some(0) | Some(4) | Some(5) | Some(6) | Some(7))
        })
    }

    /// True when this allocation is itself an element read out of an array — its defining
    /// instruction is a `toy_read_from_arr` or a reader wrapper (one whose
    /// `encapsulated_parameters` is non-empty, e.g. `fuzz.read_rand`). Such a value is not a
    /// fresh allocation: it points into the array's memory and is reclaimed by the array's
    /// deep-free, so it must never be freed independently — even if it is also used elsewhere
    /// (e.g. passed to `toy_concat`).
    fn allocation_is_array_element_read(&self, alloc: &HeapAllocation) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == *alloc.function) else {
            return false;
        };
        let callee = func
            .body
            .iter()
            .flat_map(|b| b.ins.iter())
            .find(|i| i.get_id() == alloc.alloc_ins.val)
            .and_then(|ins| match ins {
                TIR::CallExternFunction(_, name, _, _, _, _)
                | TIR::CallLocalFunction(_, name, _, _, _) => Some(name.as_ref().as_str()),
                _ => None,
            });
        let Some(callee) = callee else {
            return false;
        };
        if callee == "toy_read_from_arr" {
            return true;
        }
        let local_reader = self
            .cfg_functions
            .iter()
            .any(|f| *f.func.name == *callee && !f.parameter_encapsulates.is_empty());
        let extern_reader = self
            .alias_detector
            .external_modules
            .values()
            .flatten()
            .any(|s| s.name == callee && !s.encapsulated_parameters.is_empty());
        local_reader || extern_reader
    }

    /// True when this allocation has a use independent of being written into an array — e.g. it is a
    /// named value referenced again after the array write (`let y = ..; arr = [y]; .. y ..`), or
    /// passed elsewhere. Such a value is owned by its own binding, not the array, so its array slot
    /// must be borrowed (the array must not free it) and it is reclaimed via its own free site.
    fn allocation_used_outside_array(&self, alloc: &HeapAllocation) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == *alloc.function) else {
            return false;
        };
        let mut value_ids: HashSet<ValueId> = HashSet::new();
        value_ids.insert(alloc.alloc_ins.val);
        for (f, _, v) in &alloc.aliases {
            if *f == *alloc.function {
                value_ids.insert(*v);
            }
        }
        // Functions that write an elem param into an array param (wrappers like fuzz.write_arr),
        // keyed by name -> the elem param positions. Sourced from local CFG summaries and external
        // module summaries; the array-write builtins are the direct base case.
        let mut write_elem_positions: HashMap<String, Vec<usize>> = HashMap::new();
        for cfg_f in &self.cfg_functions {
            if !cfg_f.param_encapsulates_pairs.is_empty() {
                write_elem_positions.insert(
                    (*cfg_f.func.name).clone(),
                    cfg_f.param_encapsulates_pairs.iter().map(|(_, e)| *e).collect(),
                );
            }
        }
        // Argument positions of `name` that are array containers (the array being written/read), and
        // positions that are elements written into an array. A use of the value at any of these is
        // array plumbing, not an independent use. Sourced from local + external summaries.
        let array_arg_positions = |name: &str| -> (Vec<usize>, Vec<usize>) {
            let pairs = self
                .cfg_functions
                .iter()
                .find(|c| *c.func.name == *name)
                .map(|c| c.param_encapsulates_pairs.clone())
                .or_else(|| {
                    self.alias_detector
                        .get_external_summary(name)
                        .map(|s| s.param_encapsulates_pairs.clone())
                });
            let alias_of = self
                .cfg_functions
                .iter()
                .find(|c| *c.func.name == *name)
                .map(|c| c.returns_alias_of_parameter.clone())
                .or_else(|| {
                    self.alias_detector
                        .get_external_summary(name)
                        .map(|s| s.aliased_parameters.clone())
                })
                .unwrap_or_default();
            let mut containers = alias_of; // read wrappers (e.g. read_rand) alias the array param
            let mut elems = vec![];
            if let Some(pairs) = pairs {
                for (a, e) in pairs {
                    containers.push(a);
                    elems.push(e);
                }
            }
            (containers, elems)
        };
        // Instruction ids that are array-internal uses of this allocation — it is the array being
        // written/read (container), or the element written into an array — directly via the builtins
        // or through a wrapper call. These are array plumbing, not independent uses.
        let array_internal_ids: HashSet<ValueId> = func
            .body
            .iter()
            .flat_map(|b| b.ins.iter())
            .filter_map(|ins| match ins {
                TIR::CallExternFunction(id, name, params, _, _, _)
                    if matches!(
                        name.as_str(),
                        "toy_write_to_arr"
                            | "toy_write_to_arr_borrowed"
                            | "toy_arr_swap"
                            | "toy_arr_swap_borrowed"
                            | "toy_read_from_arr"
                            | "toy_arrlen"
                            | "toy_arr_concat"
                    ) && (params.get(0).is_some_and(|p| value_ids.contains(&p.val))
                        || params.get(1).is_some_and(|p| value_ids.contains(&p.val))) =>
                {
                    Some(*id)
                }
                TIR::CallExternFunction(id, name, args, _, _, _)
                | TIR::CallLocalFunction(id, name, args, _, _) => {
                    let (containers, elems) = array_arg_positions(name.as_str());
                    containers
                        .iter()
                        .chain(elems.iter())
                        .any(|&p| args.get(p).is_some_and(|a| value_ids.contains(&a.val)))
                        .then_some(*id)
                }
                _ => None,
            })
            .collect();
        // Any instruction that consumes this value as an operand — other than the allocation itself
        // and its array writes — keeps the value live independently of the array.
        func.body.iter().flat_map(|b| b.ins.iter()).any(|ins| {
            let id = ins.get_id();
            if id == alloc.alloc_ins.val || array_internal_ids.contains(&id) {
                return false;
            }
            Self::instruction_operand_vals(ins)
                .into_iter()
                .any(|v| value_ids.contains(&v))
        })
    }

    /// The SSA value ids this instruction consumes as operands.
    fn instruction_operand_vals(ins: &TIR) -> Vec<ValueId> {
        match ins {
            TIR::ItoF(_, v, _) | TIR::Not(_, v) | TIR::Ret(_, v) | TIR::JumpCond(_, v, _, _) => {
                vec![v.val]
            }
            TIR::ReadStructLiteral(_, v, _) => vec![v.val],
            TIR::NumericInfix(_, a, b, _) | TIR::BoolInfix(_, a, b, _) => vec![a.val, b.val],
            TIR::WriteStructLiteral(_, sv, _, nv) => vec![sv.val, nv.val],
            TIR::CallLocalFunction(_, _, args, _, _)
            | TIR::CreateStructLiteral(_, _, args)
            | TIR::Phi(_, _, args) => args.iter().map(|a| a.val).collect(),
            TIR::CallExternFunction(_, _, params, _, _, _) => {
                params.iter().map(|p| p.val).collect()
            }
            TIR::CallFuncPtr(_, fp, args, _, _) => {
                let mut v: Vec<ValueId> = vec![fp.val];
                v.extend(args.iter().map(|a| a.val));
                v
            }
            _ => vec![],
        }
    }

    /// Renames this allocation's array-write calls to their borrowed variants so the array does not
    /// free the value — the value is reclaimed via its own free site instead. Enforces disjoint
    /// single-slot ownership (each allocation owned by exactly one party).
    fn mark_array_writes_borrowed(&self, alloc: &HeapAllocation) {
        let mut builder = self.builder.borrow_mut();
        let Some(func) = builder.funcs.iter_mut().find(|f| *f.name == *alloc.function) else {
            return;
        };
        let mut value_ids: HashSet<ValueId> = HashSet::new();
        value_ids.insert(alloc.alloc_ins.val);
        for (f, _, v) in &alloc.aliases {
            if *f == *alloc.function {
                value_ids.insert(*v);
            }
        }
        for ins in func.body.iter_mut().flat_map(|b| b.ins.iter_mut()) {
            if let TIR::CallExternFunction(_, name, params, _, _, _) = ins {
                if !params.get(1).is_some_and(|p| value_ids.contains(&p.val)) {
                    continue;
                }
                match name.as_str() {
                    "toy_write_to_arr" => {
                        *name = Box::new("toy_write_to_arr_borrowed".to_string())
                    }
                    "toy_arr_swap" => *name = Box::new("toy_arr_swap_borrowed".to_string()),
                    _ => {}
                }
            }
        }
    }

    /// Marks read-back duplicate writes borrowed: a value read out of an array (`toy_read_from_arr`)
    /// and written back into an array already has an owner — its source array's other slot — so this
    /// slot only borrows it. Without this the same pointer would sit in two owned slots and the
    /// array's deep-free would reclaim it twice. Enforcing disjoint single-slot ownership at compile
    /// time is exactly what lets the runtime free skip its dedup. Runs once over every function.
    fn mark_readback_writes_borrowed(&self) {
        // Reader callees: the builtin array read plus any wrapper whose return is an element read
        // out of a param array (non-empty encapsulated-parameters summary, e.g. fuzz.read_rand).
        // A value produced by one of these is owned by its source array, so a direct write of it
        // into any array only borrows the slot.
        let mut reader_callees: HashSet<String> = HashSet::new();
        reader_callees.insert("toy_read_from_arr".to_string());
        for cfg_f in &self.cfg_functions {
            if !cfg_f.parameter_encapsulates.is_empty() {
                reader_callees.insert((*cfg_f.func.name).clone());
            }
        }
        for summaries in self.alias_detector.external_modules.values() {
            for s in summaries {
                if !s.encapsulated_parameters.is_empty() {
                    reader_callees.insert(s.name.clone());
                }
            }
        }
        let mut builder = self.builder.borrow_mut();
        for func in builder.funcs.iter_mut() {
            let readback: HashSet<ValueId> = func
                .body
                .iter()
                .flat_map(|b| b.ins.iter())
                .filter_map(|ins| match ins {
                    TIR::CallExternFunction(id, name, _, _, _, _)
                    | TIR::CallLocalFunction(id, name, _, _, _)
                        if reader_callees.contains(name.as_str()) =>
                    {
                        Some(*id)
                    }
                    _ => None,
                })
                .collect();
            for ins in func.body.iter_mut().flat_map(|b| b.ins.iter_mut()) {
                if let TIR::CallExternFunction(_, name, params, _, _, _) = ins {
                    if !params.get(1).is_some_and(|p| readback.contains(&p.val)) {
                        continue;
                    }
                    match name.as_str() {
                        "toy_write_to_arr" => {
                            *name = Box::new("toy_write_to_arr_borrowed".to_string())
                        }
                        "toy_arr_swap" => *name = Box::new("toy_arr_swap_borrowed".to_string()),
                        _ => {}
                    }
                }
            }
        }
    }

    /// Routes encapsulating-wrapper calls (e.g. `fuzz.write_arr`) to a borrowed clone when the
    /// element being stored is a value read out of an array (a borrow) and the destination array is
    /// a local, non-returned temporary. The borrowed clone (see `emit_borrowed_wrapper_variants`)
    /// does a borrowed write that frees the evicted occupant, so the destination borrows the joined
    /// slot while the source array keeps deep-free ownership of every element.
    fn mark_wrapper_writes_borrowed(&self) {
        // Callees whose result is an element read out of a param array (a borrow).
        let mut reader_callees: HashSet<String> = HashSet::new();
        reader_callees.insert("toy_read_from_arr".to_string());
        // Named encapsulating wrappers -> (arr_param_idx, elem_param_idx) pairs (excludes builtins,
        // which are handled directly by mark_readback_writes_borrowed).
        let mut wrapper_pairs: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        for cfg_f in &self.cfg_functions {
            if !cfg_f.parameter_encapsulates.is_empty() {
                reader_callees.insert((*cfg_f.func.name).clone());
            }
            if !cfg_f.param_encapsulates_pairs.is_empty() {
                wrapper_pairs.insert(
                    (*cfg_f.func.name).clone(),
                    cfg_f.param_encapsulates_pairs.clone(),
                );
            }
        }
        for summaries in self.alias_detector.external_modules.values() {
            for s in summaries {
                if !s.encapsulated_parameters.is_empty() {
                    reader_callees.insert(s.name.clone());
                }
                if !s.param_encapsulates_pairs.is_empty() {
                    wrapper_pairs.insert(s.name.clone(), s.param_encapsulates_pairs.clone());
                }
            }
        }
        if wrapper_pairs.is_empty() {
            return;
        }
        let mut builder = self.builder.borrow_mut();
        for func in builder.funcs.iter_mut() {
            // reader result value id -> the array value it was read out of (arg 0 of the reader).
            let reader_src: HashMap<ValueId, ValueId> = func
                .body
                .iter()
                .flat_map(|b| b.ins.iter())
                .filter_map(|ins| match ins {
                    TIR::CallExternFunction(id, name, args, _, _, _)
                    | TIR::CallLocalFunction(id, name, args, _, _)
                        if reader_callees.contains(name.as_str()) =>
                    {
                        args.first().map(|src| (*id, src.val))
                    }
                    _ => None,
                })
                .collect();
            let param_vals: HashSet<ValueId> = func.params.iter().map(|p| p.val).collect();
            let ret_vals: HashSet<ValueId> = func
                .body
                .iter()
                .filter_map(|b| match b.ins.last() {
                    Some(TIR::Ret(_, v)) if v.ty.is_some() => Some(v.val),
                    _ => None,
                })
                .collect();
            for ins in func.body.iter_mut().flat_map(|b| b.ins.iter_mut()) {
                let (name, args): (&mut Box<String>, &Vec<SSAValue>) = match ins {
                    TIR::CallExternFunction(_, name, args, _, _, _)
                    | TIR::CallLocalFunction(_, name, args, _, _) => (name, args),
                    _ => continue,
                };
                let Some(pairs) = wrapper_pairs.get(name.as_str()) else {
                    continue;
                };
                let route = pairs.iter().any(|&(ai, ei)| {
                    let Some(elem) = args.get(ei) else { return false };
                    let Some(dest) = args.get(ai) else { return false };
                    let Some(&src_arr) = reader_src.get(&elem.val) else {
                        return false;
                    };
                    // Self-encapsulation (read out of an array and stored back into the SAME array):
                    // the element is already owned by this array via its source slot, so the
                    // destination slot must only borrow — otherwise both slots own it and the array's
                    // deep-free reclaims it twice. This holds even when the destination is a parameter
                    // or returned array (it already owns the element), so route regardless. The
                    // dynamic-index `w == r` sub-case is resolved in `arr_swap_impl` (self-same-slot
                    // ownership preservation), which is why borrowing here cannot leak.
                    if dest.val == src_arr {
                        return true;
                    }
                    // Destination must be a local, non-returned array: a parameter or returned
                    // destination owns the element (it outlives this scope), so it must not borrow.
                    !param_vals.contains(&dest.val) && !ret_vals.contains(&dest.val)
                });
                if route {
                    *name = Box::new(format!("{}{}", name, BORROWED_WRAPPER_SUFFIX));
                }
            }
        }
    }

    /// For every encapsulating wrapper defined in this module (non-empty `param_encapsulates_pairs`),
    /// append a borrowed clone whose *encapsulating* owned write (param elem stored into param arr)
    /// becomes the (evict-freeing) `_borrowed` variant. Callers route to it via
    /// `mark_wrapper_writes_borrowed` so a value borrowed out of one array can be stored into another
    /// without the destination claiming ownership. Only the param→param store is rewritten, so any
    /// incidental array-literal construction inside the wrapper keeps its normal owned writes.
    fn emit_borrowed_wrapper_variants(&self, funcs: &mut Vec<Function>) {
        let wrapper_names: HashSet<String> = self
            .cfg_functions
            .iter()
            .filter(|f| !f.param_encapsulates_pairs.is_empty())
            .map(|f| (*f.func.name).clone())
            .collect();
        let mut clones: Vec<Function> = vec![];
        for f in funcs.iter() {
            if !wrapper_names.contains(f.name.as_ref()) {
                continue;
            }
            let param_vals: HashSet<ValueId> = f.params.iter().map(|p| p.val).collect();
            let mut clone = f.clone();
            clone.name = Box::new(format!("{}{}", f.name, BORROWED_WRAPPER_SUFFIX));
            for ins in clone.body.iter_mut().flat_map(|b| b.ins.iter_mut()) {
                if let TIR::CallExternFunction(_, name, args, _, _, _) = ins {
                    let store_of_params = args.get(0).is_some_and(|a| param_vals.contains(&a.val))
                        && args.get(1).is_some_and(|e| param_vals.contains(&e.val));
                    if !store_of_params {
                        continue;
                    }
                    match name.as_str() {
                        "toy_write_to_arr" => {
                            *name = Box::new("toy_write_to_arr_borrowed".to_string())
                        }
                        "toy_arr_swap" => *name = Box::new("toy_arr_swap_borrowed".to_string()),
                        _ => {}
                    }
                }
            }
            clones.push(clone);
        }
        funcs.extend(clones);
    }

    /// True when this allocation is an owned heap field of a struct (either an initial field of a
    /// struct literal or a value written into a field via `p.f = x`). Such a value is reclaimed by
    /// the struct's eviction free (when the field is overwritten) and its owned-field deep-free (the
    /// surviving value at struct death) — never by a per-value free here, which would double-free.
    fn allocation_written_into_struct_field(&self, alloc: &HeapAllocation) -> bool {
        // Struct-typed field values (nested structs) are not yet deep-freed at struct death, so keep
        // them on the normal pipeline (freed as their own allocation) rather than suppressing here.
        if matches!(alloc.alloc_ins.ty, Some(TirType::StructInterface(_))) {
            return false;
        }
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == *alloc.function) else {
            return false;
        };
        let mut value_ids: HashSet<ValueId> = HashSet::new();
        value_ids.insert(alloc.alloc_ins.val);
        for (f, _, v) in &alloc.aliases {
            if *f == *alloc.function {
                value_ids.insert(*v);
            }
        }
        for (f, _, v) in &alloc.refs {
            if **f == *alloc.function {
                value_ids.insert(*v);
            }
        }
        func.body.iter().flat_map(|b| b.ins.iter()).any(|ins| match ins {
            // `p.f = x`: the written value is owned by the struct.
            TIR::WriteStructLiteral(_, _, _, new_val) => value_ids.contains(&new_val.val),
            // `P{ f: x }`: a struct literal field becomes a heap field via toy_malloc_struct.
            TIR::CreateStructLiteral(_, _, fields) => {
                fields.iter().any(|fld| value_ids.contains(&fld.val))
            }
            _ => false,
        })
    }

    /// True when an element of this (local) array escapes the function — either a parameter value
    /// is stored into the array, or an element read out of the array flows into a parameter array.
    /// In both cases the element is shared with the caller, so the array must be shallow-freed (the
    /// caller reclaims the shared element); a deep-free here would double-free it.
    fn array_elements_escape(&self, array_alloc: &HeapAllocation) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == *array_alloc.function) else {
            return false;
        };
        let mut arr_ids: HashSet<ValueId> = HashSet::new();
        arr_ids.insert(array_alloc.alloc_ins.val);
        for (f, _, v) in &array_alloc.refs {
            if **f == *array_alloc.function {
                arr_ids.insert(*v);
            }
        }
        for (f, _, v) in &array_alloc.aliases {
            if *f == *array_alloc.function {
                arr_ids.insert(*v);
            }
        }
        let param_ids: HashSet<ValueId> = func.params.iter().map(|p| p.val).collect();

        // callee_name -> encapsulates pairs (arr_arg_idx, elem_arg_idx), and -> params the callee
        // returns an alias of (a read). Wrappers (fuzz.write_arr / read_rand) carry these summaries.
        // Only OWNED writes count as the element escaping into the destination's ownership. A
        // borrowed write (the destination only references the element) leaves ownership with the
        // source, so it must not drive Pattern 1/2 here — exclude the `_borrowed` variants.
        let mut enc_pairs: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        enc_pairs.insert("toy_arr_swap".to_string(), vec![(0, 1)]);
        enc_pairs.insert("toy_write_to_arr".to_string(), vec![(0, 1)]);
        // callee_name -> params whose array element is read out and returned. An element read
        // out of an array is encapsulated by that array (not an alias of it), so this uses the
        // encapsulation summary, not the whole-value alias summary. toy_read_from_arr is the
        // builtin base case (returns an element of arg 0).
        let mut elem_read: HashMap<String, Vec<usize>> = HashMap::new();
        elem_read.insert("toy_read_from_arr".to_string(), vec![0]);
        for cfg_f in &self.cfg_functions {
            if !cfg_f.param_encapsulates_pairs.is_empty() {
                enc_pairs.insert(
                    (*cfg_f.func.name).clone(),
                    cfg_f.param_encapsulates_pairs.clone(),
                );
            }
            if !cfg_f.parameter_encapsulates.is_empty() {
                elem_read.insert(
                    (*cfg_f.func.name).clone(),
                    cfg_f.parameter_encapsulates.clone(),
                );
            }
        }
        for summaries in self.alias_detector.external_modules.values() {
            for s in summaries {
                if !s.param_encapsulates_pairs.is_empty() {
                    enc_pairs.insert(s.name.clone(), s.param_encapsulates_pairs.clone());
                }
                if !s.encapsulated_parameters.is_empty() {
                    elem_read.insert(s.name.clone(), s.encapsulated_parameters.clone());
                }
            }
        }
        let call_args = |ins: &TIR| -> Option<(String, Vec<ValueId>)> {
            match ins {
                TIR::CallExternFunction(_, name, params, _, _, _) => {
                    Some(((**name).clone(), params.iter().map(|p| p.val).collect()))
                }
                TIR::CallLocalFunction(_, name, params, _, _) => {
                    Some(((**name).clone(), params.iter().map(|p| p.val).collect()))
                }
                _ => None,
            }
        };
        // Values read out of this array (an element encapsulated by it).
        let mut read_out: HashSet<ValueId> = HashSet::new();
        for ins in func.body.iter().flat_map(|b| b.ins.iter()) {
            if let Some((name, args)) = call_args(ins) {
                if let Some(ap) = elem_read.get(&name) {
                    if ap
                        .iter()
                        .any(|&k| args.get(k).is_some_and(|a| arr_ids.contains(a)))
                    {
                        read_out.insert(ins.get_id());
                    }
                }
            }
        }
        for ins in func.body.iter().flat_map(|b| b.ins.iter()) {
            if let Some((name, args)) = call_args(ins) {
                if let Some(pairs) = enc_pairs.get(&name) {
                    for &(ai, ei) in pairs {
                        let arr_arg = args.get(ai).copied();
                        let elem_arg = args.get(ei).copied();
                        // Pattern 1: a parameter value is stored into THIS array.
                        if arr_arg.is_some_and(|a| arr_ids.contains(&a))
                            && elem_arg.is_some_and(|e| param_ids.contains(&e))
                        {
                            return true;
                        }
                        // Pattern 2: an element read out of THIS array is stored into ANY OTHER
                        // array (a parameter array, or another local array via a wrapper such as
                        // fuzz.write_arr). That destination array now owns the shared element, so
                        // THIS array must be shallow-freed to avoid reclaiming it twice. The
                        // arr_ids exclusion skips self-encapsulation (read out and written back).
                        if arr_arg.is_some_and(|a| !arr_ids.contains(&a))
                            && elem_arg.is_some_and(|e| read_out.contains(&e))
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// True when an element read out of this (local) array is stored into a *destination* array that
    /// escapes the function — a parameter array or a returned array. The destination only borrows the
    /// element (read-out writes are marked borrowed), so this array still owns it; but the destination
    /// carries it beyond this scope, and the dynamic read index means this array cannot selectively
    /// free the rest. So the whole source array is left unfreed — it is encapsulated by an escaping
    /// array (invariant 1), and freeing it would leave the escaped destination with a dangling element.
    fn read_out_element_escapes_via_array(&self, array_alloc: &HeapAllocation) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == *array_alloc.function) else {
            return false;
        };
        let mut arr_ids: HashSet<ValueId> = HashSet::new();
        arr_ids.insert(array_alloc.alloc_ins.val);
        for (f, _, v) in &array_alloc.refs {
            if **f == *array_alloc.function {
                arr_ids.insert(*v);
            }
        }
        for (f, _, v) in &array_alloc.aliases {
            if *f == *array_alloc.function {
                arr_ids.insert(*v);
            }
        }
        // Arrays that leave this function: parameter arrays and (non-void) returned values.
        let mut escaping_dests: HashSet<ValueId> = func.params.iter().map(|p| p.val).collect();
        for b in &func.body {
            if let Some(TIR::Ret(_, ret_val)) = b.ins.last() {
                if ret_val.ty.is_some() {
                    escaping_dests.insert(ret_val.val);
                }
            }
        }
        // Reader callees (element read out of param k). Only the BORROWED direct writes matter
        // here: a borrowed store leaves ownership with this (source) array, so if its destination
        // escapes, this array must leak. An OWNED store (toy_write_to_arr / a write_arr wrapper)
        // transfers ownership to the destination, which then reclaims it — that is the shallow-free
        // path in `array_elements_escape`, not an escape of this array.
        let mut elem_read: HashMap<String, Vec<usize>> = HashMap::new();
        elem_read.insert("toy_read_from_arr".to_string(), vec![0]);
        let mut enc_pairs: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        enc_pairs.insert("toy_write_to_arr_borrowed".to_string(), vec![(0, 1)]);
        enc_pairs.insert("toy_arr_swap_borrowed".to_string(), vec![(0, 1)]);
        for cfg_f in &self.cfg_functions {
            if !cfg_f.parameter_encapsulates.is_empty() {
                elem_read.insert(
                    (*cfg_f.func.name).clone(),
                    cfg_f.parameter_encapsulates.clone(),
                );
            }
        }
        for summaries in self.alias_detector.external_modules.values() {
            for s in summaries {
                if !s.encapsulated_parameters.is_empty() {
                    elem_read.insert(s.name.clone(), s.encapsulated_parameters.clone());
                }
            }
        }
        let call_args = |ins: &TIR| -> Option<(String, Vec<ValueId>)> {
            match ins {
                TIR::CallExternFunction(_, name, params, _, _, _)
                | TIR::CallLocalFunction(_, name, params, _, _) => {
                    Some(((**name).clone(), params.iter().map(|p| p.val).collect()))
                }
                _ => None,
            }
        };
        let mut read_out: HashSet<ValueId> = HashSet::new();
        for ins in func.body.iter().flat_map(|b| b.ins.iter()) {
            if let Some((name, args)) = call_args(ins) {
                if let Some(ap) = elem_read.get(&name) {
                    if ap
                        .iter()
                        .any(|&k| args.get(k).is_some_and(|a| arr_ids.contains(a)))
                    {
                        read_out.insert(ins.get_id());
                    }
                }
            }
        }
        for ins in func.body.iter().flat_map(|b| b.ins.iter()) {
            if let Some((name, args)) = call_args(ins) {
                if let Some(pairs) = enc_pairs.get(&name) {
                    for &(ai, ei) in pairs {
                        let arr_arg = args.get(ai).copied();
                        let elem_arg = args.get(ei).copied();
                        if arr_arg
                            .is_some_and(|a| escaping_dests.contains(&a) && !arr_ids.contains(&a))
                            && elem_arg.is_some_and(|e| read_out.contains(&e))
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// True when this allocation is stored into one of its own function's parameters (a side-effect
    /// escape, e.g. `arr[i] = x` where `arr` is a parameter). Such values are owned by the caller's
    /// array and must not be freed inside this function.
    fn allocation_encapsulated_by_param(&self, alloc: &HeapAllocation) -> bool {
        let func = {
            let builder = self.builder.borrow();
            match builder.funcs.iter().find(|f| *f.name == *alloc.function) {
                Some(f) => f.clone(),
                None => return false,
            }
        };

        // Build (callee_name -> [(arr_param_idx, elem_param_idx)]) for direct-write verification.
        // toy_write_to_arr is the primitive; wrapper functions (e.g. fuzz.write_arr) carry this
        // relationship in their param_encapsulates_pairs summary.
        let mut enc_pairs: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        enc_pairs.insert("toy_write_to_arr".to_string(), vec![(0, 1)]);
        enc_pairs.insert("toy_write_to_arr_borrowed".to_string(), vec![(0, 1)]);
        enc_pairs.insert("toy_arr_swap".to_string(), vec![(0, 1)]);
        enc_pairs.insert("toy_arr_swap_borrowed".to_string(), vec![(0, 1)]);
        for cfg_f in &self.cfg_functions {
            if !cfg_f.param_encapsulates_pairs.is_empty() {
                enc_pairs.insert(
                    (*cfg_f.func.name).clone(),
                    cfg_f.param_encapsulates_pairs.clone(),
                );
            }
        }
        for summaries in self.alias_detector.external_modules.values() {
            for s in summaries {
                if !s.param_encapsulates_pairs.is_empty() {
                    enc_pairs.insert(s.name.clone(), s.param_encapsulates_pairs.clone());
                }
            }
        }

        alloc.encapsulators.iter().any(|(enc_func, _, enc_val)| {
            if enc_func.as_str() != func.name.as_ref() {
                return false;
            }
            if !func.params.iter().any(|p| p.val == *enc_val) {
                return false;
            }
            // Verify that alloc.alloc_ins.val or any intermediate encapsulator of alloc (other
            // than enc_val itself) is the element arg in some encapsulating call where enc_val is
            // the array arg. Including intermediate encapsulators handles the transitive case where
            // an alias of alloc is stored into the param (e.g. str_h → arr_h → %elem → p1).
            // Excluding enc_val itself avoids infinite recursion and spurious matches.
            let mut direct_write_seeds: HashSet<ValueId> = HashSet::from([alloc.alloc_ins.val]);
            let alias_ids: HashSet<ValueId> = alloc
                .aliases
                .iter()
                .filter(|(f, _, _)| f.as_str() == func.name.as_ref())
                .map(|(_, _, v)| *v)
                .collect();
            for (ef, _, ev) in &alloc.encapsulators {
                if ef.as_str() == func.name.as_ref()
                    && *ev != *enc_val
                    && *ev != alloc.alloc_ins.val
                    && !alias_ids.contains(ev)
                {
                    direct_write_seeds.insert(*ev);
                }
            }

            func.body.iter().flat_map(|b| b.ins.iter()).any(|ins| {
                let (callee_name, args): (&str, &Vec<SSAValue>) = match ins {
                    TIR::CallLocalFunction(_, name, args, _, _) => (name.as_ref(), args),
                    TIR::CallExternFunction(_, name, args, _, _, _) => (name.as_ref(), args),
                    TIR::WriteStructLiteral(_, struct_val, _, new_val) => {
                        if struct_val.val == *enc_val {
                            let mut visited = HashSet::new();
                            return self.value_may_match_seed_via_phi(
                                &func,
                                new_val.val,
                                &direct_write_seeds,
                                &mut visited,
                            );
                        }
                        return false;
                    }
                    _ => return false,
                };
                let Some(pairs) = enc_pairs.get(callee_name) else {
                    return false;
                };
                pairs.iter().any(|&(arr_idx, elem_idx)| {
                    args.get(arr_idx).is_some_and(|a| a.val == *enc_val)
                        && args.get(elem_idx).is_some_and(|e| {
                            let mut visited = HashSet::new();
                            self.value_may_match_seed_via_phi(
                                &func,
                                e.val,
                                &direct_write_seeds,
                                &mut visited,
                            )
                        })
                })
            })
        })
    }

    /// Determines if a given allocation escapes the function it was created in, escapes the program as a whole, or dies in the function
    fn allocation_escapes(&self, alloc: &HeapAllocation) -> EscapeType {
        let builder = self.builder.borrow();
        let func = builder
            .funcs
            .iter()
            .find(|f| *f.name == *alloc.function)
            .unwrap();
        let protected_ids = self.allocation_protected_values_in_function(alloc, func.name.as_ref());
        let is_param = func.params.iter().any(|p| p.val == alloc.alloc_ins.val);
        //params always freed by the caller
        if is_param {
            return EscapeType::EscapesFunction;
        }

        // A value stored into a parameter array (side-effect escape) is owned by the caller's
        // array, which escapes this function. By invariant 1 it must not be freed here; its
        // reclamation rides on the swap-evicted value (toy_arr_swap return) or the array's
        // deep-free at its owner.
        if self.allocation_encapsulated_by_param(alloc) {
            return EscapeType::EscapesFunction;
        }

        // If the allocation was created by a call that returns an alias of one of its
        // arguments (and has no owned fields of its own), it is not a fresh allocation —
        // it points into existing memory and must not be freed separately.
        if self.allocation_is_returned_param_alias(alloc) {
            return EscapeType::EscapesProgram;
        }

        //alloc is returned
        for b in &func.body {
            if let Some(TIR::Ret(_, a)) = b.ins.last() {
                // Void returns use a sentinel SSAValue { val: 0, ty: None }; skip them to avoid
                // false positives when a param or other allocation happens to have val == 0.
                if a.ty.is_none() {
                    continue;
                }
                if a.val == alloc.alloc_ins.val {
                    return EscapeType::EscapesFunction;
                }
                if protected_ids.contains(&a.val) {
                    let ret_is_phi = func
                        .body
                        .iter()
                        .flat_map(|b| b.ins.iter())
                        .any(|ins| ins.get_id() == a.val && matches!(ins, TIR::Phi(_, _, _)));
                    if !ret_is_phi {
                        return EscapeType::EscapesFunction;
                    }
                }
                let mut visited = HashSet::new();
                if self.value_may_be_allocation_via_phi(&func, a.val, alloc, &mut visited) {
                    return EscapeType::EscapesFunction;
                }
            }
        }

        for b in &func.body {
            for i in &b.ins {
                match i {
                    TIR::CallExternFunction(_, callee_name, p, _, _, doesnt_take_ownership) => {
                        if Self::is_free_func_name(callee_name.as_ref()) {
                            continue;
                        }
                        for (idx, arg) in p.iter().enumerate() {
                            let is_alloc_ref = protected_ids.contains(&arg.val) || {
                                let mut visited = HashSet::new();
                                self.value_may_be_allocation_via_phi(
                                    &func,
                                    arg.val,
                                    alloc,
                                    &mut visited,
                                )
                            };

                            if is_alloc_ref {
                                if let Some(summary) = self
                                    .alias_detector
                                    .get_external_summary(callee_name.as_ref())
                                {
                                    if summary.escaped_parameters.contains(&idx) {
                                        return EscapeType::EscapesModule;
                                    }
                                } else if !doesnt_take_ownership.get(idx).copied().unwrap_or(false)
                                {
                                    return EscapeType::EscapesProgram;
                                }
                            }
                        }
                    }
                    _ => continue,
                };
            }
        }

        return EscapeType::DoesNotEscape;
    }

    /// determines if he block or any of its children reference the given allocation, has a cycle guard
    fn block_children_reference_allocation(
        &self,
        func: &CFGFunction,
        cfg_b: &CFGBlock,
        alloc: &HeapAllocation,
        tracked_blocks: &HashSet<BlockId>,
        visited: &mut HashSet<BlockId>,
        is_root: bool,
    ) -> bool {
        if visited.contains(&cfg_b.block) {
            return false;
        }
        visited.insert(cfg_b.block);

        // only check refs in non-root blocks (successors, not the candidate block itself)
        if !is_root {
            if tracked_blocks.contains(&cfg_b.block) {
                return true;
            }
        }

        if cfg_b.possible_output_blocks.is_empty() {
            return false;
        }

        for possible_output_block in &cfg_b.possible_output_blocks {
            let child = func
                .cfg_blocks
                .iter()
                .find(|b| b.block == *possible_output_block)
                .unwrap();
            if self.block_children_reference_allocation(
                func,
                child,
                alloc,
                tracked_blocks,
                visited,
                false,
            ) {
                return true;
            }
        }
        false
    }
    /// finds the function and the exact SSAValue where the allocation is initialized and must be freed
    fn find_owning_function(&self, alloc: &HeapAllocation) -> (String, SSAValue) {
        let mut current_func = alloc.function.clone();
        let mut current_val = alloc.alloc_ins.clone();
        let mut visited = HashSet::new();

        loop {
            if !visited.insert(current_func.to_string()) {
                return ((*current_func).clone(), current_val);
            }

            // find all callers that receive a value from current_func via CallLocalFunction
            // this is ludicrous and needs to be refactored into like 5 separate things.
            let callers: Vec<(Function, Block, SSAValue)> = {
                let builder = self.builder.borrow();
                //also I have seen egyptian hyreoglphyics that make more sense then 4 statements inside 8 lambdas and 2 flat maps.
                builder
                    .funcs
                    .iter()
                    .flat_map(|f| {
                        f.body.iter().flat_map(|b| {
                            b.ins.iter().filter_map(|i| {
                                if let TIR::CallLocalFunction(ret_id, name, _, _, ret_type) = i {
                                    if **name == *current_func {
                                        return Some((
                                            f.clone(),
                                            b.clone(),
                                            SSAValue {
                                                val: *ret_id,
                                                ty: Some(ret_type.clone()),
                                            },
                                        ));
                                    }
                                }
                                if let TIR::CallExternFunction(ret_id, name, _, _, ret_type, _) = i
                                {
                                    if **name == *current_func {
                                        return Some((
                                            f.clone(),
                                            b.clone(),
                                            SSAValue {
                                                val: *ret_id,
                                                ty: Some(ret_type.clone()),
                                            },
                                        ));
                                    }
                                }
                                None
                            })
                        })
                    })
                    .collect()
            };

            if callers.is_empty() {
                // nobody called us, we are the owner
                return ((*current_func).clone(), current_val);
            }

            let mut next_hop: Option<(Box<String>, SSAValue)> = None;
            for (caller_func, caller_block, new_val) in callers {
                // check if THIS function also escapes it
                let test_alloc = HeapAllocation {
                    function: caller_func.name.clone(),
                    alloc_ins: new_val.clone(),
                    block: caller_block.id,
                    refs: alloc
                        .refs
                        .iter()
                        .filter(|(f, _, _)| *f == caller_func.name)
                        .cloned()
                        .collect(),
                    allocation_id: alloc.allocation_id,
                    aliases: BTreeSet::new(),       //temp
                    encapsulators: BTreeSet::new(), //temp
                };
                if self.allocation_escapes(&test_alloc) == EscapeType::DoesNotEscape {
                    return ((*caller_func.name).clone(), new_val);
                }
                if next_hop.is_none() {
                    next_hop = Some((caller_func.name.clone(), new_val));
                }
            }

            if let Some((next_func, next_val)) = next_hop {
                current_func = next_func;
                current_val = next_val;
            } else {
                return ((*current_func).clone(), current_val);
            }
        }
    }
    ///takes an allocation and its owning function and marks the point where the free call should be inserted.
    fn process_non_escaping_allocation(
        &self,
        cfg_func: &CFGFunction,
        func: &Function,
        alloc: &HeapAllocation,
        insertion_points: &mut Vec<(String, BlockId, ValueId, SSAValue, String)>,
    ) {
        if !self.function_has_ssa(func, alloc.alloc_ins.val) {
            return;
        }
        let free_func = self.alloc_type_to_free_func(alloc);
        let origin_block_id = alloc.block;
        let Some(origin_cfg_block) = cfg_func
            .cfg_blocks
            .iter()
            .find(|b| b.block == origin_block_id)
        else {
            return;
        };

        let mut visited: HashSet<BlockId> = HashSet::new();
        let mut tracked_blocks =
            self.allocation_tracked_blocks_in_function(alloc, func.name.as_ref());
        if std::env::var("TOY_DEBUG_CTLA").is_ok() {
            eprintln!("[CTLA_DEBUG] alloc val={} func={} block={} refs={:?} tracked={:?}", alloc.alloc_ins.val, func.name, origin_block_id, alloc.refs, tracked_blocks);
        }

        // Also track blocks where encapsulator values are used — if an encapsulator
        // (e.g. an outer array holding this allocation) is still live in a child block,
        // this allocation must not be freed until the encapsulator is.
        for (enc_func, _, enc_vid) in &alloc.encapsulators {
            if enc_func.as_str() == func.name.as_ref() {
                let enc_set = HashSet::from([*enc_vid]);
                for block in &func.body {
                    if block
                        .ins
                        .iter()
                        .any(|ins| self.instruction_uses_any_value(ins, &enc_set))
                    {
                        tracked_blocks.insert(block.id);
                    }
                }
            }
        }

        let has_child_refs = self.block_children_reference_allocation(
            cfg_func,
            origin_cfg_block,
            alloc,
            &tracked_blocks,
            &mut visited,
            true,
        );

        if !has_child_refs {
            if self.block_returns_allocation_or_alias(func, origin_block_id, alloc) {
                return;
            }
            let insertion_idx = if free_func == "toy_free_arr"
                || free_func == "toy_deep_free_arr"
                || free_func == "toy_deep_free_arr_evicted"
            {
                func.body
                    .iter()
                    .find(|b| b.id == origin_block_id)
                    .map(|b| b.ins.len().saturating_sub(1))
                    .unwrap()
            } else {
                self.free_insertion_index_for_block(func, origin_block_id, alloc)
            };
            insertion_points.push((
                *func.name.clone(),
                origin_block_id,
                insertion_idx,
                alloc.alloc_ins.clone(),
                free_func,
            ));
            return;
        }

        // Compute blocks dominated by origin to find exit points for the allocation's scope.
        let dominated = self.blocks_dominated_by(cfg_func, origin_block_id);

        for cfg_block in &cfg_func.cfg_blocks {
            if !dominated.contains(&cfg_block.block) {
                continue;
            }
            let is_exit = cfg_block.possible_output_blocks.is_empty()
                || cfg_block
                    .possible_output_blocks
                    .iter()
                    .all(|s| !dominated.contains(s));
            if !is_exit {
                continue;
            }

            let Some(block) = func.body.iter().find(|b| b.id == cfg_block.block) else {
                continue;
            };

            // Blocks with no successors that aren't Ret (e.g. panic) — skip
            if cfg_block.possible_output_blocks.is_empty()
                && !matches!(block.ins.last(), Some(TIR::Ret(_, _)))
            {
                continue;
            }

            if self.block_returns_allocation_or_alias(func, cfg_block.block, alloc) {
                continue;
            }

            let insertion_idx =
                if free_func == "toy_free_arr"
                    || free_func == "toy_deep_free_arr"
                    || free_func == "toy_deep_free_arr_evicted"
                {
                    block.ins.len().saturating_sub(1)
                } else {
                    self.free_insertion_index_for_block(func, cfg_block.block, alloc)
                };

            insertion_points.push((
                *func.name.clone(),
                cfg_block.block,
                insertion_idx,
                alloc.alloc_ins.clone(),
                free_func.clone(),
            ));
        }
    }
    /// Returns the set of blocks dominated by `origin` in the given CFG function.
    /// A block B is dominated by `origin` if every path from the entry block to B
    /// passes through `origin`.
    fn blocks_dominated_by(
        &self,
        cfg_func: &CFGFunction,
        origin: BlockId,
    ) -> HashSet<BlockId> {
        let entry = cfg_func.cfg_blocks[0].block;
        let mut reachable: HashSet<BlockId> = HashSet::new();
        let mut queue: VecDeque<BlockId> = VecDeque::new();
        if entry != origin {
            reachable.insert(entry);
            queue.push_back(entry);
        }
        while let Some(b) = queue.pop_front() {
            if let Some(cfg_b) = cfg_func.cfg_blocks.iter().find(|cb| cb.block == b) {
                for &succ in &cfg_b.possible_output_blocks {
                    if succ != origin && !reachable.contains(&succ) {
                        reachable.insert(succ);
                        queue.push_back(succ);
                    }
                }
            }
        }
        let all_blocks: HashSet<BlockId> =
            cfg_func.cfg_blocks.iter().map(|b| b.block).collect();
        let mut dominated: HashSet<BlockId> =
            all_blocks.difference(&reachable).cloned().collect();
        dominated.insert(origin);
        dominated
    }

    /// determines if a given ssa value is in the given function body or parameters
    fn function_has_ssa(&self, func: &Function, value_id: ValueId) -> bool {
        if func.params.iter().any(|p| p.val == value_id) {
            return true;
        }
        func.body
            .iter()
            .any(|b| b.ins.iter().any(|ins| ins.get_id() == value_id))
    }

    /// Returns the instruction at the given function, block, and value, note the inputs are id's NOT indexes
    /// Resolves the constant value of an `IConst` SSA value within a function (used to decode the
    /// element-type code carried by a `toy_arr_swap` call).
    fn resolve_iconst(&self, function_name: &str, value_id: ValueId) -> Option<i64> {
        self.builder
            .borrow()
            .funcs
            .iter()
            .find(|f| *f.name == function_name)
            .and_then(|f| {
                f.body.iter().flat_map(|b| b.ins.iter()).find_map(|ins| {
                    if let TIR::IConst(id, v, _) = ins {
                        (*id == value_id).then_some(*v)
                    } else {
                        None
                    }
                })
            })
    }
    fn get_alloc_ins(
        &self,
        function_name: &str,
        block_id: BlockId,
        value_id: ValueId,
    ) -> Option<TIR> {
        self.builder
            .borrow()
            .funcs
            .iter()
            .find(|f| *f.name == function_name)
            .and_then(|f| f.body.iter().find(|b| b.id == block_id))
            .and_then(|b| b.ins.iter().find(|ins| ins.get_id() == value_id))
            .cloned()
    }
    /// crude doubleplusungood function that tests if a given function name matches 3 known to return arrays
    fn is_array_allocation_call_name(&self, name: &str) -> bool {
        return name == "toy_malloc_arr" || name == "std::sys::argv" || name == "toy_sys_get_argv";
    }
    /// tries to determine if a function returns an array by name, this is bad however and should in future just ask the TIRBuilder
    fn function_returns_array_allocation(&self, function_name: &str) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == function_name) else {
            return false;
        };

        for block in &func.body {
            if let Some(TIR::Ret(_, ret_ssa)) = block.ins.last() {
                if let Some(ins) = block.ins.iter().find(|i| i.get_id() == ret_ssa.val) {
                    if let TIR::CallExternFunction(_, f_box, _, _, _, _) = ins {
                        if self.is_array_allocation_call_name(f_box) {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }
    fn function_returns_struct_allocation(&self, function_name: &str) -> bool {
        let builder = self.builder.borrow();
        let Some(func) = builder.funcs.iter().find(|f| *f.name == function_name) else {
            return false;
        };

        for block in &func.body {
            if let Some(TIR::Ret(_, ret_ssa)) = block.ins.last() {
                if let Some(ins) = block.ins.iter().find(|i| i.get_id() == ret_ssa.val) {
                    if let TIR::CallExternFunction(_, f_box, _, _, _, _) = ins {
                        if f_box.as_ref() == "toy_malloc_struct" {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }
    /// matches the allocation type to the type of free needed (regular or array), returns that function name
    fn alloc_type_to_free_func(&self, alloc: &HeapAllocation) -> String {
        let alloc_ins = self.get_alloc_ins(&alloc.function, alloc.block, alloc.alloc_ins.val);

        let Some(alloc_ins) = alloc_ins else {
            return "toy_free".to_string();
        };

        match alloc_ins {
            TIR::CallExternFunction(_, f_box, params, _, ret_type, _) => {
                // A toy_arr_swap result is an evicted array element; its free variant is decided by
                // the element-type code injected as the 4th arg (str=0, struct=8, nested-array=4..7).
                if f_box.as_ref() == "toy_arr_swap" || f_box.as_ref() == "toy_arr_swap_borrowed" {
                    // The evicted value may be reported as 0 (self-write-back or borrowed slot), so
                    // use the null-safe evicted free variants.
                    let code = params
                        .get(3)
                        .and_then(|p| self.resolve_iconst(&alloc.function, p.val));
                    return match code {
                        Some(4) | Some(5) | Some(6) | Some(7) => {
                            "toy_deep_free_arr_evicted".to_string()
                        }
                        _ => "toy_free_evicted".to_string(),
                    };
                }
                //that argv thing is hacky but I dont know how to say under the hood it calls toy_malloc_arr
                if self.is_array_allocation_call_name(f_box.as_ref()) {
                    // The array owns its elements: deep-free reclaims any still-live slot contents
                    // (survivors) when the element type is heap-owned. toy_malloc_arr encodes the
                    // element-type code as its 2nd arg (str=0, struct=8, nested-array=4..7).
                    let elem_code = if f_box.as_ref() == "toy_malloc_arr" {
                        params
                            .get(1)
                            .and_then(|p| self.resolve_iconst(&alloc.function, p.val))
                    } else {
                        None
                    };
                    let heap_elem = matches!(
                        elem_code,
                        Some(0) | Some(4) | Some(5) | Some(6) | Some(7)
                    );
                    // Deep-free reclaims survivors for heap-element arrays — but only when the array
                    // uniquely owns its elements. If an element escapes to a parameter (shared with
                    // the caller), shallow-free; the caller reclaims the shared element.
                    return if heap_elem && !self.array_elements_escape(alloc) {
                        "toy_deep_free_arr".to_string()
                    } else {
                        "toy_free_arr".to_string()
                    };
                } else if f_box.as_ref() == "toy_malloc_struct" {
                    return "toy_free_struct".to_string();
                }
                // Non-allocator extern funcs: check return type to pick the right free
                if matches!(ret_type, TirType::StructInterface(_)) {
                    return "toy_free_struct".to_string();
                }
                return "toy_free".to_string();
            }
            // For local function calls that return heap-allocated values (strings),
            TIR::CallLocalFunction(_, callee_name, _, _, _) => {
                if self.function_returns_array_allocation(callee_name.as_ref()) {
                    return "toy_deep_free_arr".to_string();
                } else if self.function_returns_struct_allocation(callee_name.as_ref()) {
                    return "toy_free_struct".to_string();
                }
                return "toy_free".to_string();
            }
            TIR::CallFuncPtr(_, _, _, _, _) => {
                // Pointer calls do not encode array-vs-scalar ownership metadata here.
                // Default to scalar free to avoid dropping heap strings from lambdas.
                //this will cause errors
                return "toy_free".to_string();
            }
            // A struct-field value surfaced by an overwrite (eviction). A struct-typed field needs
            // the struct free (8-byte size prefix); str / array fields free as plain pointers.
            TIR::ReadStructLiteral(_, _, _) => {
                if matches!(alloc.alloc_ins.ty, Some(TirType::StructInterface(_))) {
                    return "toy_free_struct".to_string();
                }
                return "toy_free".to_string();
            }
            _ => return "toy_free".to_string(),
        };
    }

    /// Look up return_owned_fields for a struct allocation by finding its allocating call
    /// and checking the external summary for that function.
    fn get_return_owned_fields_for_alloc(&self, func_name: &str, alloc_val: ValueId) -> Vec<OwnedField> {
        let builder = self.builder.borrow();
        let func = builder.funcs.iter().find(|f| *f.name == func_name);
        let Some(func) = func else { return vec![] };
        let alloc_ins = func
            .body
            .iter()
            .flat_map(|b| b.ins.iter())
            .find(|ins| ins.get_id() == alloc_val);
        match alloc_ins {
            // A local struct (`P{..}`) lowers to `toy_malloc_struct(size, struct_literal)`. Its owned
            // heap fields come straight from the struct literal, so it gets the same field deep-free
            // treatment as a struct returned from a function.
            Some(TIR::CallExternFunction(_, callee_name, args, _, _, _))
                if callee_name.as_ref() == "toy_malloc_struct" =>
            {
                let Some(struct_lit) = args.get(1) else {
                    return vec![];
                };
                let mut fields = self.alias_detector.owned_fields_of_struct_value(
                    func,
                    struct_lit.val,
                    &self.cfg_functions,
                );
                // Struct-typed fields (nested structs) need a recursive struct free that isn't
                // implemented yet; freeing them as plain pointers would crash/leak. Drop them so
                // only str / array fields are deep-freed (the rest stay on the normal pipeline).
                if let Some(TIR::CreateStructLiteral(_, TirType::StructInterface(types), _)) = func
                    .body
                    .iter()
                    .flat_map(|b| b.ins.iter())
                    .find(|ins| ins.get_id() == struct_lit.val)
                {
                    fields.retain(|f| {
                        !matches!(types.get(f.index), Some(TirType::StructInterface(_)))
                    });
                }
                fields
            }
            Some(TIR::CallLocalFunction(_, callee_name, _, _, _))
            | Some(TIR::CallExternFunction(_, callee_name, _, _, _, _)) => {
                // Check local cfg_functions first
                if let Some(cfg_f) = self.cfg_functions.iter().find(|f| *f.func.name == **callee_name) {
                    if !cfg_f.return_owned_fields.is_empty() {
                        return cfg_f.return_owned_fields.clone();
                    }
                }
                // Check external summaries
                if let Some(summary) = self.alias_detector.get_external_summary(callee_name.as_ref()) {
                    return summary.return_owned_fields.clone();
                }
                vec![]
            }
            _ => vec![],
        }
    }

    /// runs the full pipeline to mark (or intentionally leak) a given allocation
    fn process_allocation(
        &mut self,
        alloc: &mut HeapAllocation,
        insertion_points: &mut Vec<(String, BlockId, ValueId, SSAValue, String)>,
    ) {
        let func = {
            let builder = self.builder.borrow();
            builder
                .funcs
                .iter()
                .find(|f| *f.name == *alloc.function)
                .cloned()
                .unwrap()
        };
        self.alias_detector
            .find_aliases_and_encapsulators(alloc, &mut self.cfg_functions);
        let is_param = func.params.iter().any(|p| p.val == alloc.alloc_ins.val);
        if is_param {
            return;
        }
        let cfg_func = self
            .cfg_functions
            .iter()
            .find(|f| f.func.name == func.name)
            .unwrap();
        // A value stored into a parameter array is owned by the caller's array (invariant 1);
        // do not free it here. The swap-evicted value carries reclamation instead.
        if self.allocation_encapsulated_by_param(&alloc) {
            return;
        }
        // A value read out of an array is owned by that array (encapsulated by it); it is
        // reclaimed by the array's deep-free and must never be freed here, even when it is also
        // used elsewhere (e.g. fed to toy_concat) — that use is a borrow, not ownership.
        if self.allocation_is_array_element_read(&alloc) {
            return;
        }
        // An array whose read-out element is carried off by an escaping destination array (a param
        // or returned array borrows it) is itself encapsulated by that escaping array: it must be
        // left unfreed, else the escaped destination would hold a dangling element (invariant 1).
        if self.read_out_element_escapes_via_array(&alloc) {
            return;
        }
        // A value stored into a local array is owned by that array: it is reclaimed by the array's
        // deep-free (if it survives) or by the swap-eviction free (if overwritten), never by a
        // per-element free — which would double-free. Exception: if the value also has an
        // independent use (it outlives the array slot, e.g. a named local read after the write),
        // the array only borrows it — mark its writes borrowed and free it via its own site.
        if self.allocation_written_into_array(&alloc) {
            if self.allocation_used_outside_array(&alloc) {
                self.mark_array_writes_borrowed(&alloc);
            } else {
                return;
            }
        }
        // A value owned by a struct field is reclaimed via the struct's eviction + owned-field
        // deep-free, never by a per-value free here.
        if self.allocation_written_into_struct_field(&alloc) {
            return;
        }
        let escape_type = self.allocation_escapes(&alloc);
        if escape_type == EscapeType::EscapesProgram || escape_type == EscapeType::EscapesModule {
            //at this pont let it leak, it it escapes the program
            return;
        } else if escape_type == EscapeType::DoesNotEscape {
            //if in this branch, the allocation dies in ths function
            self.process_non_escaping_allocation(cfg_func, &func, &alloc, insertion_points);
        } else {
            let (owning_func_name, owning_val) = self.find_owning_function(&alloc);

            let owning_func = {
                let builder = self.builder.borrow();
                builder
                    .funcs
                    .iter()
                    .find(|f| *f.name == owning_func_name)
                    .cloned()
                    .unwrap()
            };

            // find the block containing the call site (where owning_val was defined)
            let owning_block_id = owning_func
                .body
                .iter()
                .find(|b| b.ins.iter().any(|i| i.get_id() == owning_val.val))
                .unwrap()
                .id;

            let owned_alloc = HeapAllocation {
                block: owning_block_id,
                function: Box::new(owning_func_name.clone()),
                alloc_ins: owning_val,
                allocation_id: alloc.allocation_id,
                refs: alloc
                    .refs
                    .iter()
                    .filter(|(f, _, _)| **f == owning_func_name)
                    .cloned()
                    .collect(),
                aliases: alloc
                    .aliases
                    .iter()
                    .filter(|(f, _, _)| *f == owning_func_name)
                    .cloned()
                    .collect(),
                encapsulators: alloc
                    .encapsulators
                    .iter()
                    .filter(|(f, _, _)| *f == owning_func_name)
                    .cloned()
                    .collect(),
            };

            // Re-run the encapsulation checks rooted at the owning function: a call-returned
            // value (e.g. an array literal element that is a function call) has its array write
            // in the owner, not in the function that ran toy_malloc, so the checks above missed it.
            if self.allocation_encapsulated_by_param(&owned_alloc) {
                return;
            }
            if self.allocation_written_into_array(&owned_alloc) {
                if self.allocation_used_outside_array(&owned_alloc) {
                    self.mark_array_writes_borrowed(&owned_alloc);
                } else {
                    return;
                }
            }
            if self.allocation_written_into_struct_field(&owned_alloc) {
                return;
            }

            let owning_cfg_func = self
                .cfg_functions
                .iter()
                .find(|f| *f.func.name == owning_func_name)
                .unwrap();

            self.process_non_escaping_allocation(
                owning_cfg_func,
                &owning_func,
                &owned_alloc,
                insertion_points,
            );
        }
    }
    /// For each local function, finds pairs (arr_param_idx, elem_param_idx) where the function
    /// directly calls toy_write_to_arr(param[arr], param[elem], ...).
    fn populate_param_encapsulates_pairs(funcs: &mut Vec<CFGFunction>) {
        for cfg_f in funcs.iter_mut() {
            let mut pairs: Vec<(usize, usize)> = vec![];
            for block in &cfg_f.func.body {
                for ins in &block.ins {
                    if let TIR::CallExternFunction(_, name, wp, _, _, _) = ins {
                        if matches!(
                            name.as_str(),
                            "toy_write_to_arr"
                                | "toy_write_to_arr_borrowed"
                                | "toy_arr_swap"
                                | "toy_arr_swap_borrowed"
                        ) && wp.len() >= 2
                        {
                            let arr_idx = cfg_f.func.params.iter().position(|p| p.val == wp[0].val);
                            let elem_idx = cfg_f.func.params.iter().position(|p| p.val == wp[1].val);
                            if let (Some(ai), Some(ei)) = (arr_idx, elem_idx) {
                                let pair = (ai, ei);
                                if !pairs.contains(&pair) {
                                    pairs.push(pair);
                                }
                            }
                        }
                    }
                }
            }
            cfg_f.param_encapsulates_pairs = pairs;
        }
    }

    fn populate_parameter_escape_summary(&self, funcs: Vec<CFGFunction>) -> Vec<CFGFunction> {
        let mut new_funcs: Vec<CFGFunction> = vec![];
        for cfg_func in funcs {
            let mut new_cfg = cfg_func.clone();
            for (idx, param) in new_cfg.func.params.iter().enumerate() {
                if param.ty != Some(TirType::Ptr) {
                    continue;
                }

                let seeds = HashSet::from([param.val]);
                let mut param_escapes_program = false;
                for b in &new_cfg.func.body {
                    for ins in &b.ins {
                        if let TIR::CallExternFunction(
                            _,
                            callee,
                            args,
                            _,
                            _,
                            doesnt_take_ownership,
                        ) = ins
                        {
                            for (arg_idx, arg) in args.iter().enumerate() {
                                let mut visited = HashSet::new();
                                if self.value_may_match_seed_via_phi(
                                    &new_cfg.func,
                                    arg.val,
                                    &seeds,
                                    &mut visited,
                                ) {
                                    if let Some(summary) =
                                        self.alias_detector.get_external_summary(callee.as_ref())
                                    {
                                        if summary.escaped_parameters.contains(&arg_idx) {
                                            param_escapes_program = true;
                                        }
                                    } else if !doesnt_take_ownership
                                        .get(arg_idx)
                                        .copied()
                                        .unwrap_or(false)
                                    {
                                        param_escapes_program = true;
                                    }
                                }
                            }
                        }
                    }
                }

                if param_escapes_program {
                    new_cfg.parameter_escapes.push(idx);
                }
            }
            new_cfg.parameter_escapes.sort_unstable();
            new_cfg.parameter_escapes.dedup();
            new_funcs.push(new_cfg);
        }
        return new_funcs;
    }
    /// Runs CTLA Analysis on the given Builder, returns a vec of functions containing the processed code, or an error.
    pub fn analyze(&mut self, builder: TirBuilder) -> Result<Vec<Function>, ToyError> {
        let module_name = Driver::get_current_file_path()
            .and_then(|p| {
                std::path::Path::new(&p)
                    .file_stem()
                    .and_then(|s| s.to_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "module".to_string());
        let mut external_modules = self.alias_detector.external_modules.clone();
        self.builder = Rc::new(RefCell::new(builder));
        self.alias_detector = AliasAndEncapsulationTracker::new(&self.builder);
        // toy_malloc_struct copies a struct to the heap and returns the heap pointer.
        // The returned heap allocation captures all pointer values stored in the struct
        // (param 1), so the return value aliases param 1.
        external_modules
            .entry("__builtins__".to_string())
            .or_default()
            .push(FunctionSummary::new(
                "toy_malloc_struct".to_string(),
                vec![1],
                vec![],
                vec![],
                vec![],
                vec![],
            ));
        self.alias_detector.set_external_modules(external_modules);
        self.cfg_functions.clear();

        //build per-function CFG graphs
        {
            let mut builder = self.builder.borrow_mut();
            for f in &mut builder.funcs {
                let mut cfg_f = CFGFunction::new(f.to_owned());
                cfg_f.calc_cfg();
                self.cfg_functions.push(cfg_f);
            }
        }
        self.builder.borrow_mut().eliminate_trivial_phis();
        self.build_phi_index();
        self.alias_detector
            .populate_return_alias_parameter_summaries(&mut self.cfg_functions);
        self.alias_detector
            .populate_return_owned_fields(&mut self.cfg_functions);
        self.cfg_functions = self.populate_parameter_escape_summary(self.cfg_functions.clone());
        CTLA::populate_param_encapsulates_pairs(&mut self.cfg_functions);
        // Enforce disjoint single-slot ownership before per-allocation processing: a value read out
        // of an array and written back is borrowed in the new slot (its source slot owns it).
        self.mark_readback_writes_borrowed();
        // Same idea, but where the store goes through an encapsulating wrapper (e.g. fuzz.write_arr):
        // route the call to a borrowed clone so the destination borrows the joined slot.
        self.mark_wrapper_writes_borrowed();
        let mut unique_allocations = self.builder.borrow().detect_unique_heap_allocations();
        let mut insertion_points: Vec<(String, BlockId, ValueId, SSAValue, String)> = vec![];
        let len = unique_allocations.len();
        for (i, a) in unique_allocations.iter_mut().enumerate() {
            self.process_allocation(a, &mut insertion_points);
            print!(
                "process allocations for {i} completed, {:.2}%\r",
                (i as f64) / (len as f64) * 100.0
            );
            std::io::stdout().flush().unwrap();
        }
        //this is all terrible, but just clears the line
        print!(
            "                                                                                                    \r"
        );
        std::io::stdout().flush().unwrap();
        let dedup_set: HashSet<_> = insertion_points.into_iter().collect();
        insertion_points = dedup_set.into_iter().collect();

        let mut coalesced: HashMap<(String, BlockId, ValueId, String), (usize, SSAValue)> =
            HashMap::new();
        for (name, bid, idx, val, free_name) in insertion_points {
            let key = (name.clone(), bid, val.val, free_name.clone());
            if let Some((existing_idx, _)) = coalesced.get(&key) {
                if idx > *existing_idx {
                    coalesced.insert(key, (idx, val));
                }
            } else {
                coalesced.insert(key, (idx, val));
            }
        }
        insertion_points = coalesced
            .into_iter()
            .map(|((name, bid, _, free_name), (idx, val))| (name, bid, idx, val, free_name))
            .collect();

        let free_sort_rank = |free_name: &str| {
            if free_name == "toy_free_arr"
                || free_name == "toy_deep_free_arr"
                || free_name == "toy_deep_free_arr_evicted"
            {
                1usize
            } else {
                0usize
            }
        };
        insertion_points.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| free_sort_rank(a.4.as_str()).cmp(&free_sort_rank(b.4.as_str())))
                .then_with(|| a.3.val.cmp(&b.3.val))
        });
        for (name, bid, vid, val, free_name) in insertion_points {
            if free_name == "toy_free_struct" {
                // Check if the struct allocation has owned fields that need freeing
                let owned_fields = self.get_return_owned_fields_for_alloc(&name, val.val);
                if !owned_fields.is_empty() {
                    let inserted = self.builder.borrow_mut().splice_struct_field_frees_before(
                        name.clone(),
                        bid,
                        vid,
                        val.clone(),
                        &owned_fields,
                    );
                    // The struct free goes after the field frees
                    self.builder.borrow_mut().splice_free_before(
                        name,
                        bid,
                        vid + inserted,
                        val,
                        free_name,
                    );
                    continue;
                }
            }
            self.builder
                .borrow_mut()
                .splice_free_before(name, bid, vid, val, free_name);
        }
        let build_dir = Driver::get_build_dir();
        let mut summaries: Vec<FunctionSummary> = vec![];
        for func in &self.cfg_functions {
            summaries.push(FunctionSummary::new(
                *func.func.name.clone(),
                func.returns_alias_of_parameter.clone(),
                func.parameter_encapsulates.clone(),
                func.parameter_escapes.clone(),
                func.return_owned_fields.clone(),
                func.param_encapsulates_pairs.clone(),
            ));
        }

        let mut hasher = ahash::AHasher::default();
        use std::hash::Hasher;
        hasher.write(self.original_text.as_deref().unwrap_or("").as_bytes());
        let hash = format!("{:x}", hasher.finish());

        let schema = CTLASchema::new(CTLA_SCHEMA_VERSION, summaries, hash, module_name.clone());
        let serialized = serde_json::to_string(&schema).unwrap(); //should fix ?

        let _ = fs::create_dir_all(&build_dir);
        let res = fs::write(format!("{}/{}.ctla", build_dir, module_name), serialized);
        match res {
            Err(e) => {
                eprintln!(
                    "[ERROR] Could not write module {} with error {:?}",
                    module_name, e
                )
            }
            _ => {}
        };

        // A call returning an alias of its argument (e.g. read_rand) is not a fresh allocation, so
        // exclude these from the statistics entirely — they neither own memory nor escape anything.
        let real_allocations: Vec<&HeapAllocation> = unique_allocations
            .iter()
            .filter(|a| !self.allocation_is_returned_param_alias(a))
            .collect();
        let alloc_count = real_allocations.len() as u64;
        let alias_count: u64 = real_allocations.iter().map(|a| a.aliases.len() as u64).sum();
        let encap_count: u64 = real_allocations
            .iter()
            .map(|a| a.encapsulators.len() as u64)
            .sum();
        let mut escape_func_count = 0u64;
        let mut escape_mod_count = 0u64;
        let mut escape_prog_count = 0u64;
        for a in real_allocations {
            let is_param = {
                let builder = self.builder.borrow();
                builder
                    .funcs
                    .iter()
                    .find(|f| *f.name == *a.function)
                    .map(|f| f.params.iter().any(|p| p.val == a.alloc_ins.val))
                    .unwrap_or(false)
            };
            if is_param {
                escape_func_count += 1;
                continue;
            }
            match self.allocation_escapes(a) {
                EscapeType::EscapesFunction => escape_func_count += 1,
                EscapeType::EscapesModule => escape_mod_count += 1,
                EscapeType::EscapesProgram => escape_prog_count += 1,
                EscapeType::DoesNotEscape => {}
            }
        }
        let escape_func_pct = if alloc_count > 0 {
            escape_func_count as f64 / alloc_count as f64
        } else {
            0.0
        };
        let escape_mod_pct = if alloc_count > 0 {
            escape_mod_count as f64 / alloc_count as f64
        } else {
            0.0
        };
        let escape_prog_pct = if alloc_count > 0 {
            escape_prog_count as f64 / alloc_count as f64
        } else {
            0.0
        };
        self.stats = Some(CTLAStats {
            alloc_count,
            alias_count,
            encap_count,
            escape_func_pct,
            escape_mod_pct,
            fp_iters: self.alias_detector.total_fp_iters.get(),
            escape_prog_pct: Some(escape_prog_pct),
            total_bytes: None,
            malloc_calls: None,
            lifetime_mean_ns: None,
            lifetime_median_ns: None,
            lifetime_min_ns: None,
            lifetime_max_ns: None,
        });

        let mut out_funcs = self.builder.borrow().funcs.clone();
        self.emit_borrowed_wrapper_variants(&mut out_funcs);
        return Ok(out_funcs);
    }
}

#[cfg(test)]
mod tests;
