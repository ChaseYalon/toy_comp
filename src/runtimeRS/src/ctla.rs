use crate::stub::DEBUG_HEAP;
use std::env;
use std::io;
use std::io::Write;
use std::{collections::HashMap, os::raw::c_void, usize};
use crate::ToyPtr;
#[derive(Debug, Clone)]
pub struct DebugHeap {
    ///ptr -> size
    pub map: HashMap<i64, i64>,
    pub total_live_allocations: i64,
    pub total_allocations: i64,
}
impl DebugHeap {
    pub fn new() -> DebugHeap {
        return DebugHeap {
            map: HashMap::new(),
            total_live_allocations: 0,
            total_allocations: 0,
        };
    }
}

pub fn _toy_malloc_debug(size: usize) -> *mut c_void {
    let buff = unsafe { libc::malloc(size) };
    let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
    heap.map.insert(buff as i64, size as i64);
    heap.total_live_allocations += 1;
    heap.total_allocations += 1;
    return buff;
}
#[unsafe(no_mangle)]
pub fn toy_free(buff: *mut c_void) {
    if buff.is_null() {
        eprintln!("[ERROR] Tried to free a null buffer");
        unsafe { libc::abort() };
    }
    let val = env::var("TOY_DEBUG");
    if let Ok(v) = val {
        if v == "TRUE" {
            let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
            if let Some(&value) = heap.map.get(&(buff as i64)) {
                if value >= 0 {
                    heap.total_live_allocations -= 1;
                }
            }
            heap.map.insert(buff as i64, -1);
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
            if let Some(&value) = heap.map.get(&(real_ptr as i64)) {
                if value >= 0 {
                    heap.total_live_allocations -= 1;
                }
            }
            heap.map.insert(real_ptr as i64, -1);
        }
    }
    unsafe { libc::free(real_ptr) };
}
#[unsafe(no_mangle)]
pub fn _print_debug_heap() {
    println!("{:#?}", DEBUG_HEAP.get().unwrap().lock().unwrap().map);
    println!(
        "Total Allocations: {}",
        DEBUG_HEAP.get().unwrap().lock().unwrap().total_allocations
    );
    println!(
        "Total Live Allocations: {}",
        DEBUG_HEAP
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .total_live_allocations
    );
}

#[unsafe(no_mangle)]
pub fn _check_pointer(buff: *mut c_void) {
    if buff.is_null() {
        eprintln!("\n[ERROR] Null pointer detected");
        eprintln!("\nFAIL_TEST");
        panic!();
    }

    let v = env::var("TOY_DEBUG");
    if v.is_err() || v.unwrap() != "TRUE" {
        return;
    }
    if let Some(value) = DEBUG_HEAP
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .map
        .get(&(buff as i64))
    {
        if *value == -1 {
            eprintln!(
                "[ERROR] Use-after-free detected! Pointer {:p} was already freed",
                buff
            );
            println!("\nFAIL_TEST");
            io::stdout().flush().ok();
            io::stderr().flush().ok();
            panic!();
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
