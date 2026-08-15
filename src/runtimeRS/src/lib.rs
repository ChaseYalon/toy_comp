//The ownership contract must be followed at all times
//  variables that are heap allocated and external to rust must use toy_malloc*
//  variables that are internal to the rust code may use the borrow checker

mod builtins;
mod ctla;
mod stub;
mod toy_std;
mod values;
pub type ToyPtr = i64;
use std::{alloc::{GlobalAlloc, Layout}, sync::Mutex};

//makes sure that rust allocations can be freed from C
struct LibcAllocator;
///total number of bytes the program has allocated
pub static TOTAL_ALLOCATION_SIZES: Mutex<u64> = Mutex::new(0);


#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn GC_realloc(ptr: *mut libc::c_void, size: usize) -> *mut libc::c_void;
    /// Returns the base of the GC object containing `ptr`, or null if `ptr` is not GC memory.
    fn GC_base(ptr: *mut libc::c_void) -> *mut libc::c_void;
    fn GC_memalign(align: usize, size: usize) -> *mut libc::c_void;
}

/// Alignment `GC_malloc` (like `libc::malloc`) already guarantees. Anything stricter has to go
/// through `GC_memalign`, so this is the threshold both allocation paths test against.
#[cfg(target_os = "linux")]
const GC_MALLOC_ALIGN: usize = 16;

/// Honors `layout`'s alignment, which plain `GC_malloc` does not promise beyond
/// [`GC_MALLOC_ALIGN`].
#[cfg(target_os = "linux")]
unsafe fn gc_alloc(layout: Layout) -> *mut u8 {
    if layout.align() > GC_MALLOC_ALIGN {
        return unsafe { GC_memalign(layout.align(), layout.size()) as *mut u8 };
    }
    ctla::toy_gc_malloc(layout.size()) as *mut u8
}

/// Under TOY_GC every Rust-side allocation must come from the collector, not just the ones the
/// language hands back. A `ToyArr`'s `arr: Vec<i64>` holds the array's live element pointers, and
/// Boehm only traces registers, the stack, static data and its own heap — element buffers left in
/// libc memory are invisible to it, so their contents get collected while the array still points at
/// them. Routing the global allocator fixes that for every container at once.
unsafe impl GlobalAlloc for LibcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[cfg(target_os = "linux")]
        if ctla::gc_enabled() {
            return unsafe { gc_alloc(layout) };
        }
        unsafe { libc::malloc(layout.size()) as *mut u8 }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // The latch flips inside `stub::init`, so anything the Rust runtime allocated before ctors
        // ran is libc memory that still needs a real free. `GC_base` distinguishes the two: non-null
        // means the collector owns it and will reclaim it itself.
        #[cfg(target_os = "linux")]
        if ctla::gc_enabled() {
            if !unsafe { GC_base(ptr as *mut libc::c_void) }.is_null() {
                return;
            }
        }
        unsafe { libc::free(ptr as *mut libc::c_void) }
    }

    // `layout` is only consulted on the GC path, which does not exist off Linux.
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[cfg(target_os = "linux")]
        if ctla::gc_enabled() {
            if !unsafe { GC_base(ptr as *mut libc::c_void) }.is_null() {
                // `GC_realloc` promises no more alignment than `GC_malloc`, so an over-aligned
                // block has to be re-allocated and copied rather than grown in place. The old
                // block is left to the collector — under GC nothing here frees.
                if layout.align() > GC_MALLOC_ALIGN {
                    let new_layout =
                        match Layout::from_size_align(new_size, layout.align()) {
                            Ok(l) => l,
                            Err(_) => return std::ptr::null_mut(),
                        };
                    let out = unsafe { gc_alloc(new_layout) };
                    if !out.is_null() {
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                ptr,
                                out,
                                layout.size().min(new_size),
                            )
                        };
                    }
                    return out;
                }
                return unsafe { GC_realloc(ptr as *mut libc::c_void, new_size) as *mut u8 };
            }
        }
        unsafe { libc::realloc(ptr as *mut libc::c_void, new_size) as *mut u8 }
    }
}

#[global_allocator]
static GLOBAL: LibcAllocator = LibcAllocator;
