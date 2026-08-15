use crate::stub::DEBUG_HEAP;
use std::env;
use std::io;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
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
#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn GC_malloc(size: usize) -> *mut c_void;
}
/// Backing storage for [`toy_debug`]. Written exactly once, by [`init_toy_debug`].
static TOY_DEBUG: AtomicBool = AtomicBool::new(false);
/// Reads `TOY_DEBUG` from the environment. Called once from `stub::init` before any toy code runs;
/// nothing writes this again for the rest of the process.
pub fn init_toy_debug() {
    TOY_DEBUG.store(
        env::var("TOY_DEBUG").as_deref() == Ok("TRUE"),
        Ordering::Relaxed,
    );
}
/// Whether debug heap tracking is on. Every allocation/free path reads this instead of the env var,
/// so the whole run sees one consistent value. Relaxed is sufficient: the only write happens in the
/// ctor, before any reader exists.
#[inline(always)]
pub fn toy_debug() -> bool {
    TOY_DEBUG.load(Ordering::Relaxed)
}
/// Backing storage for [`gc_enabled`]. Written exactly once, by [`init_toy_gc`].
#[cfg(target_os = "linux")]
static TOY_GC: AtomicBool = AtomicBool::new(false);
/// Set when a memory fault is detected. Under TOY_GC only a null pointer reaches here — a
/// tombstoned pointer means CTLA advised a free the collector did not act on, which is a premature
/// static free rather than a fault — and the run continues so the collector, not CTLA, is measured.
static SHOULD_EXIT: AtomicBool = AtomicBool::new(false);
/// Records a detected memory fault. Returns true only for the first fault of the run, so the GC
/// path reports once instead of re-reporting every later touch of the same pointer. Callers print
/// their diagnostic and then call [`fault_exit`].
fn mark_should_exit() -> bool {
    !SHOULD_EXIT.swap(true, Ordering::Relaxed)
}
/// Ends the run at a detected fault, unless a collector is holding the memory up.
///
/// The shadow-heap lookup that leads here is identical in both configurations, so the GC never
/// skips detection work a CTLA run pays for; dying at the fault only ever makes the CTLA run do
/// *less*, which cannot flatter the collector in a benchmark. Continuing is not an option off-GC:
/// the program goes on reading and writing reclaimed memory, corrupts glibc's arena, and dies in
/// `malloc` with the original fault buried under a pile of unrelated aborts.
fn fault_exit() -> ! {
    println!("\nFAIL_TEST");
    io::stdout().flush().ok();
    io::stderr().flush().ok();
    // Strictly after the marker is out and flushed: capturing a backtrace can itself abort (libgcc's
    // unwinder calls `abort()` when it cannot find `PT_GNU_EH_FRAME`, which our linker output does
    // not always carry), and losing the failure marker to a diagnostic is worse than losing the
    // diagnostic.
    print_backtrace_if_requested();
    std::process::exit(1);
}
/// Dumps a backtrace when `TOY_UAF_BACKTRACE=TRUE`. Never called before the caller has flushed
/// whatever it needs the outside world to see - see [`fault_exit`].
pub fn print_backtrace_if_requested() {
    if env::var("TOY_UAF_BACKTRACE").as_deref() == Ok("TRUE") {
        eprintln!("{}", std::backtrace::Backtrace::force_capture());
        io::stderr().flush().ok();
    }
}
/// Reads `TOY_GC` from the environment. Called once from `stub::init` before any toy code runs, and
/// before the first allocation: the global allocator consults [`gc_enabled`] on every call, and
/// reading the environment there would allocate re-entrantly.
pub fn init_toy_gc() {
    #[cfg(target_os = "linux")]
    TOY_GC.store(
        env::var("TOY_GC").is_ok_and(|v| v.to_lowercase() == "true"),
        Ordering::Relaxed,
    );
}
/// Whether TOY_GC is on. GC support only exists on Linux (the vendored `libgc.a` is Linux-only), so
/// this is unconditionally false elsewhere. Every allocation/free path checks this instead of reading
/// the env var directly, so the on/off behavior stays consistent across the runtime.
#[inline(always)]
pub fn gc_enabled() -> bool {
    #[cfg(target_os = "linux")]
    {
        TOY_GC.load(Ordering::Relaxed)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
/// Raw GC-backed allocation, shared by every allocation site that needs to route through the
/// collector (scalars/strings/structs via `_toy_malloc_debug`, arrays via `toy_malloc_arr`).
/// Only ever called when `gc_enabled()` is true, which is only possible on Linux.
pub fn toy_gc_malloc(size: usize) -> *mut c_void {
    #[cfg(target_os = "linux")]
    {
        unsafe { GC_malloc(size) }
    }
    #[cfg(not(target_os = "linux"))]
    {
        unreachable!("GC is only supported on Linux")
    }
}
/// Shadow-heap key for `ptr`, with the bits inverted so the key does not look like a pointer.
///
/// Under TOY_GC the map's own buffer is GC memory, and Boehm scans it conservatively. Because the
/// map never removes entries — a free tombstones the slot to `-1` rather than dropping it — raw
/// pointer keys would pin every allocation the program ever made, so nothing would ever be
/// collected and GC mode would silently measure nothing. This is Boehm's standard hidden-pointer
/// idiom; it is safe because the shadow heap is diagnostics, never ownership: anything the program
/// can still reach is held by a real reference (stack, register, or an array's element buffer)
/// that the collector does trace.
#[inline(always)]
pub fn heap_key(ptr: i64) -> i64 {
    !ptr
}
pub fn _toy_malloc_debug(size: usize) -> *mut c_void {
    let buff = if gc_enabled() {
        toy_gc_malloc(size)
    } else {
        unsafe { libc::malloc(size) }   
    };
    // Bookkeeping runs for GC-backed allocations too: a CTLA-vs-GC benchmark is only meaningful if
    // both configurations pay the same shadow-heap cost.
    if toy_debug() {
        let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
        heap.map
            .insert(heap_key(buff as i64), (size as i64, Instant::now()));
        heap.total_live_allocations += 1;
        heap.total_allocations += 1;
        *TOTAL_ALLOCATION_SIZES.lock().unwrap() += size as u64;
    }
    return buff;
}
#[unsafe(no_mangle)]
pub fn toy_free(buff: *mut c_void) {
    // Must precede the bookkeeping below: that stamps the slot as freed (-1), which this would
    // then read back as a use-after-free on every single free.
    _check_pointer(buff);
    if toy_debug() {
        let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
        if let Some(&(size, alloc_time)) = heap.map.get(&heap_key(buff as i64)) {
            if size >= 0 {
                heap.total_live_allocations -= 1;
                heap.lifetimes_ns.push(alloc_time.elapsed().as_nanos() as u64);
            }
        }
        heap.map.insert(heap_key(buff as i64), (-1, Instant::now()));
    }
    if gc_enabled() {
        return
    }
    unsafe { libc::free(buff) };
}
#[unsafe(no_mangle)]
pub fn toy_free_struct(ptr: ToyPtr) {
    if (ptr as *mut c_void).is_null() {
        if mark_should_exit() {
            eprintln!("\n[ERROR] Null pointer detected");
        }
        if !gc_enabled() {
            fault_exit();
        }
        return;
    }
    // The shadow heap (and the allocator) key structs on the base pointer, while the program holds
    // the body pointer (base + 8). Check the base so struct double-frees/UAFs are actually detected
    // instead of slipping through to a second libc::free and corrupting the real heap.
    let real_ptr = unsafe { (ptr as *mut u8).sub(8) as *mut c_void };
    _check_pointer(real_ptr);
    // Reclaim any fields the struct still owns (invariant 5): static death sites already freed
    // theirs (and cleared the bits), so this only fires for structs dying at runtime, e.g. elements
    // of a deep-freed returned array.
    crate::builtins::struct_free_owned_fields(ptr);
    // Drop any per-field ownership entry keyed on the body pointer (see builtins::STRUCT_FIELD_OWNED).
    crate::builtins::toy_struct_forget(ptr);
    if toy_debug() {
        let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
        if let Some(&(size, alloc_time)) = heap.map.get(&heap_key(real_ptr as i64)) {
            if size >= 0 {
                heap.total_live_allocations -= 1;
                heap.lifetimes_ns.push(alloc_time.elapsed().as_nanos() as u64);
            }
        }
        heap.map
            .insert(heap_key(real_ptr as i64), (-1, Instant::now()));
    }
    if gc_enabled() {
        return
    }
    unsafe { libc::free(real_ptr) };
}

#[unsafe(no_mangle)]
pub fn _print_debug_heap() {
    let heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
    for (key, (size, _)) in &heap.map {
        if *size >= 0 {
            println!("  {:#x}, {}", heap_key(*key), size);
        }
    }
    println!("Total Allocations: {}", heap.total_allocations);
    println!("Total Live Allocations: {}", heap.total_live_allocations);
}

#[unsafe(no_mangle)]
pub fn _check_pointer(buff: *mut c_void) {
    // A null deref is a fault whether or not the shadow heap is on, so this precedes the TOY_DEBUG
    // gate; only the use-after-free check below needs the heap.
    if buff.is_null() {
        if mark_should_exit() {
            eprintln!("\n[ERROR] Null pointer detected");
        }
        if !gc_enabled() {
            fault_exit();
        }
        return;
    }
    if !toy_debug() {
        return;
    }
    // A tombstone records that CTLA asked for a free, which under TOY_GC releases nothing: the
    // memory stays valid and collector-owned, so touching it afterwards is a premature *static*
    // free, not a fault. Reporting those here is what buried the real GC faults in noise. The
    // lookup still runs so the two configurations do the same shadow-heap work.
    let freed = {
        let heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
        matches!(heap.map.get(&heap_key(buff as i64)), Some(&(-1, _)))
    };
    if freed && !gc_enabled() {
        if mark_should_exit() {
            eprintln!(
                "[ERROR] Use-after-free detected! Pointer {:p} was already freed",
                buff
            );
            io::stdout().flush().ok();
            io::stderr().flush().ok();
        }
        // The guard is dropped before reporting: `fault_exit` runs no destructors, so holding the
        // shadow-heap lock across it would leave it poisoned for anything that outlives the exit.
        fault_exit();
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
