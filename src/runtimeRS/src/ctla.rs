use crate::stub::DEBUG_HEAP;
use std::env;
use std::io;
use std::io::Write;
use std::time::Instant;
use std::{collections::HashMap, os::raw::c_void, usize};
use crate::ToyPtr;
use crate::TOTAL_ALLOCATION_SIZES;
#[derive(Debug)]
pub struct DebugHeap {
    /// ptr -> (size, alloc_time)
    pub map: HashMap<i64, (i64, Instant)>,
    pub total_live_allocations: i64,
    pub total_allocations: i64,
    /// Lifetime in nanoseconds for each freed allocation
    pub lifetimes_ns: Vec<u64>,
    /// (element ptr, element ToyType as i64) for every heap scalar written into an array via
    /// `toy_write_to_arr`. CTLA cannot statically track values stored into a caller-owned array
    /// through a library wrapper, so these are swept at program exit and freed if still live.
    pub written_elems: Vec<(i64, i64)>,
}
impl DebugHeap {
    pub fn new() -> DebugHeap {
        return DebugHeap {
            map: HashMap::new(),
            total_live_allocations: 0,
            total_allocations: 0,
            lifetimes_ns: Vec::new(),
            written_elems: Vec::new(),
        };
    }
}

pub fn _toy_malloc_debug(size: usize) -> *mut c_void {
    let buff = unsafe { libc::malloc(size) };
    let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
    heap.map.insert(buff as i64, (size as i64, Instant::now()));
    heap.total_live_allocations += 1;
    heap.total_allocations += 1;
    *TOTAL_ALLOCATION_SIZES.lock().unwrap() += size as u64;
    return buff;
}
#[unsafe(no_mangle)]
pub fn toy_free(buff: *mut c_void) {
    if buff.is_null() {
        eprintln!("[ERROR] Tried to free a null buffer");
        std::process::exit(1);
    }
    _check_pointer(buff);
    let val = env::var("TOY_DEBUG");
    if let Ok(v) = val {
        if v == "TRUE" {
            let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
            if let Some(&(size, alloc_time)) = heap.map.get(&(buff as i64)) {
                if size >= 0 {
                    heap.total_live_allocations -= 1;
                    heap.lifetimes_ns.push(alloc_time.elapsed().as_nanos() as u64);
                }
            }
            heap.map.insert(buff as i64, (-1, Instant::now()));
        }
    }
    unsafe { libc::free(buff) };
}
#[unsafe(no_mangle)]
pub fn toy_free_struct(ptr: ToyPtr) {
    _check_pointer(ptr as *mut c_void);
    let real_ptr = unsafe { (ptr as *mut u8).sub(8) as *mut c_void };
    let val = env::var("TOY_DEBUG");
    if let Ok(v) = val {
        if v == "TRUE" {
            let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
            if let Some(&(size, alloc_time)) = heap.map.get(&(real_ptr as i64)) {
                if size >= 0 {
                    heap.total_live_allocations -= 1;
                    heap.lifetimes_ns.push(alloc_time.elapsed().as_nanos() as u64);
                }
            }
            heap.map.insert(real_ptr as i64, (-1, Instant::now()));
        }
    }
    unsafe { libc::free(real_ptr) };
}
/// Sweeps elements written into arrays, freeing any that are still live. Runs at program exit
/// after all of user_main's CTLA-inserted frees have executed, so any element CTLA already freed
/// is marked dead (size == -1) and skipped here — making this conflict-free with CTLA.
#[unsafe(no_mangle)]
pub fn _free_written_elems() {
    if env::var("TOY_DEBUG").as_deref() != Ok("TRUE") {
        return;
    }
    let pending: Vec<(i64, i64)> = {
        let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
        std::mem::take(&mut heap.written_elems)
    };
    enum FreeKind { Scalar, Struct, Arr }
    for (ptr, ty) in pending {
        if ptr == 0 {
            continue;
        }
        let (check_ptr, kind) = match crate::values::ToyType::try_from(ty) {
            Ok(crate::values::ToyType::Str) => (ptr, FreeKind::Scalar),
            Ok(crate::values::ToyType::Struct) => (ptr - 8, FreeKind::Struct),
            Ok(crate::values::ToyType::StrArr) => (ptr, FreeKind::Arr),
            _ => continue,
        };
        let live = matches!(
            DEBUG_HEAP.get().unwrap().lock().unwrap().map.get(&check_ptr),
            Some(&(size, _)) if size >= 0
        );
        if !live {
            continue;
        }
        match kind {
            FreeKind::Scalar => toy_free(ptr as *mut c_void),
            FreeKind::Struct => toy_free_struct(ptr as ToyPtr),
            FreeKind::Arr => super::builtins::toy_free_arr(ptr as ToyPtr),
        }
    }
}

#[unsafe(no_mangle)]
pub fn _print_debug_heap() {
    let heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
    for (ptr, (size, _)) in &heap.map {
        if *size >= 0 {
            println!("  {:#x}, {}", ptr, size);
        }
    }
    println!("Total Allocations: {}", heap.total_allocations);
    println!("Total Live Allocations: {}", heap.total_live_allocations);
}

#[unsafe(no_mangle)]
pub fn _check_pointer(buff: *mut c_void) {
    if buff.is_null() {
        eprintln!("\n[ERROR] Null pointer detected");
        eprintln!("\nFAIL_TEST");
        std::process::exit(1);
    }

    let v = env::var("TOY_DEBUG");
    if v.is_err() || v.unwrap() != "TRUE" {
        return;
    }
    if let Some(&(size, _)) = DEBUG_HEAP
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .map
        .get(&(buff as i64))
    {
        if size == -1 {
            eprintln!(
                "[ERROR] Use-after-free detected! Pointer {:p} was already freed",
                buff
            );
            println!("\nFAIL_TEST");
            io::stdout().flush().ok();
            io::stderr().flush().ok();
            std::process::exit(1);
        }
    }
}

#[unsafe(no_mangle)]
pub fn should_fail() -> i64 {
    let res = DEBUG_HEAP
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .total_live_allocations
        != 0;
    return if res { 1 } else { 0 };
}
