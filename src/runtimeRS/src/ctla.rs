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
}
impl DebugHeap {
    pub fn new() -> DebugHeap {
        return DebugHeap {
            map: HashMap::new(),
            total_live_allocations: 0,
            total_allocations: 0,
            lifetimes_ns: Vec::new(),
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
    if (ptr as *mut c_void).is_null() {
        eprintln!("\n[ERROR] Null pointer detected");
        eprintln!("\nFAIL_TEST");
        std::process::exit(1);
    }
    // The shadow heap (and the allocator) key structs on the base pointer, while the program holds
    // the body pointer (base + 8). Check the base so struct double-frees/UAFs are actually detected
    // instead of slipping through to a second libc::free and corrupting the real heap.
    let real_ptr = unsafe { (ptr as *mut u8).sub(8) as *mut c_void };
    _check_pointer(real_ptr);
    // Drop any per-field ownership entry keyed on the body pointer (see builtins::STRUCT_FIELD_OWNED).
    crate::builtins::toy_struct_forget(ptr);
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
            if env::var("TOY_UAF_BACKTRACE").as_deref() == Ok("TRUE") {
                eprintln!("{}", std::backtrace::Backtrace::force_capture());
            }
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
