use crate::stub::{DEBUG_HEAP};
use std::env;
use std::{collections::HashMap, os::raw::c_void, usize};
#[derive(Debug, Clone)]
pub struct DebugHeap {
    ///ptr -> size
    pub Map: HashMap<i64, i64>,
    pub TotalLiveAllocations: i64,
    pub TotalAllocations: i64,
}
#[unsafe(no_mangle)]
pub fn DebugHeap_create() -> *mut DebugHeap {
    let dbh = Box::new(DebugHeap {
        Map: HashMap::new(),
        TotalAllocations: 0,
        TotalLiveAllocations: 0,
    });
    return Box::into_raw(dbh);
}

#[unsafe(no_mangle)]
pub fn DebugHeap_free(d: *mut DebugHeap) {
    if d.is_null() {
        return;
    }
    unsafe {
        if (*d).TotalLiveAllocations != 0 {
            eprintln!(
                "[WARN] There are {} live allocations remaining, at heap deallocations",
                (*d).TotalLiveAllocations
            );
        }
        drop(Box::from_raw(d)); // actually frees the heap memory
    }
}

#[unsafe(no_mangle)]
pub fn ToyMallocDebug(size: usize, d: *mut DebugHeap) -> *mut c_void {
    let buff = unsafe { libc::malloc(size) };
    unsafe {
        (*d).Map.insert(buff as i64, size as i64);
        (*d).TotalLiveAllocations += 1;
        (*d).TotalAllocations += 1;
    };

    return buff;
}

#[unsafe(no_mangle)]
pub fn toy_free(buff: *mut c_void) {
    if buff.is_null() {
        eprintln!("[ERROR] Tried to free a null buffer");
        unsafe { libc::abort() };
    }
    let val = env::var("TOY_DEBUG");
    match val {
        Ok(v) => {
            if v == "TRUE" {
                // Only decrement if this pointer was actually tracked (has a non-negative value)
                if let Some(value) = unsafe { (*DEBUG_HEAP).Map.get(&(buff as i64)) } {
                    if *value >= 0 {
                        unsafe { (*DEBUG_HEAP).TotalLiveAllocations -= 1 };
                    }
                }
                unsafe { &mut *DEBUG_HEAP }.Map.insert(buff as i64, -1);
            }
        }
        _ => {}
    };
    unsafe { libc::free(buff) };
}

#[unsafe(no_mangle)]
pub fn _PrintDebug_heap(d: *mut DebugHeap) {
    println!("{:#?}", unsafe { &*d }.Map);
    println!("Total Allocations: {}", unsafe { &*d }.TotalAllocations);
    println!(
        "Total Live Allocations: {}",
        unsafe { &*d }.TotalLiveAllocations
    );
}

#[unsafe(no_mangle)]
pub fn _CheckUseAfterFree(buff: *mut c_void) {
    if buff.is_null() {
        eprintln!("\n[ERROR] Null pointer detected");
        eprintln!("\nFAIL_TEST");
        panic!();
    }

    let v = env::var("TOY_DEBUG");
    if v.is_err() || v.unwrap() != "TRUE" {
        return;
    }
    if let Some(value) = unsafe { &*DEBUG_HEAP }.Map.get(&(buff as i64)) {
        if *value == -1 {
            eprintln!(
                "[ERROR] Use-after-free detected! Pointer {:p} was already freed",
                buff
            );
            println!("\nFAIL_TEST");
            use std::io::Write;
            std::io::stdout().flush().ok();
            std::io::stderr().flush().ok();
            panic!();
        }
    }
}

#[unsafe(no_mangle)]
pub fn should_fail() -> i64 {
    let res = unsafe { &*DEBUG_HEAP }.TotalLiveAllocations != 0;
    return if res { 1 } else { 0 };
}
