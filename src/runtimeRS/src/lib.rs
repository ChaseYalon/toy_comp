//The ownership contract must be followed at all times
//  variables that are heap allocated and external to rust must use toy_malloc*
//  variables that are internal to the rust code may use the borrow checker

mod builtins;
mod ctla;
mod stub;
mod toy_std;
mod values;
pub type ToyPtr = i64;
use std::alloc::{GlobalAlloc, Layout};
//makes sure that rust allocations can be freed from C
struct LibcAllocator;

unsafe impl GlobalAlloc for LibcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { libc::malloc(layout.size()) as *mut u8 }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { libc::free(ptr as *mut libc::c_void) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        unsafe { libc::realloc(ptr as *mut libc::c_void, new_size) as *mut u8 }
    }
}

#[global_allocator]
static GLOBAL: LibcAllocator = LibcAllocator;
