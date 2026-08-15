use crate::{TOTAL_ALLOCATION_SIZES, ctla::{_print_debug_heap, init_toy_debug, DebugHeap}};
use ctor::ctor;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use std::fs;
use std::backtrace::Backtrace;
unsafe extern "C" {
    fn user_main() -> i64;
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CTLAStats {
    alloc_count: u64,
    alias_count: u64,
    encap_count: u64,
    escape_func_pct: f64,
    escape_mod_pct: f64,
    fp_iters: u64,
    ctla_wallclock_ns: u64,
    escape_prog_pct: Option<f64>,
    total_bytes: Option<u64>,
    malloc_calls: Option<u64>,
    lifetime_mean_ns: Option<u64>,
    lifetime_median_ns: Option<u64>,
    lifetime_min_ns: Option<u64>,
    lifetime_max_ns: Option<u64>,
    program_runtime_ns: Option<u64>,
    // Runtime-only: absent from the compiler-emitted blob, so it must stay Option to deserialize.
    gc_is_on: Option<bool>,
}

unsafe extern "C" {
    /// Byte array emitted by the compiler containing the magic prefix followed by JSON-serialized CTLAStats
    static __toy_ctla_stats_blob: u8;
}

const CTLA_MAGIC: &[u8] = b"__TOY_CTLA_STATS__";

fn read_ctla_stats() -> Option<CTLAStats> {
    let ptr = &raw const __toy_ctla_stats_blob as *const u8;
    let magic_len = CTLA_MAGIC.len();
    let prefix = unsafe { std::slice::from_raw_parts(ptr, magic_len) };
    if prefix != CTLA_MAGIC {
        return None;
    }
    let json_start = unsafe { ptr.add(magic_len) };
    let mut len = 0usize;
    while unsafe { *json_start.add(len) } != 0 {
        len += 1;
    }
    let json_bytes = unsafe { std::slice::from_raw_parts(json_start, len) };
    let json_str = std::str::from_utf8(json_bytes).ok()?;
    serde_json::from_str(json_str).ok()
}

#[unsafe(no_mangle)]
pub static DEBUG_HEAP: OnceLock<Mutex<DebugHeap>> = OnceLock::new();

static PROGRAM_START: OnceLock<std::time::Instant> = OnceLock::new();

/// Milliseconds since process start, or 0 before the ctor has run.
pub(crate) fn program_elapsed_ms() -> u64 {
    return PROGRAM_START
        .get()
        .map(|start| start.elapsed().as_millis() as u64)
        .unwrap_or(0);
}

#[unsafe(no_mangle)]
pub static mut GLOBAL_ARGC: i64 = 0;

#[unsafe(no_mangle)]
pub static mut GLOBAL_ARGV: *mut *mut libc::c_char = std::ptr::null_mut();

#[cfg(windows)]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(
        filter: Option<unsafe extern "system" fn(*mut std::ffi::c_void) -> i32>,
    ) -> *mut std::ffi::c_void;
}

fn report_segfault_and_exit() -> ! {
    use std::io::Write;
    eprintln!("\n[FATAL] toy_lang runtime aborted: memory access violation (segfault)");
    // The marker goes out and gets flushed before the backtrace is attempted. `Backtrace` can abort
    // inside libgcc's unwinder when the linked image has no `PT_GNU_EH_FRAME`, and it was doing so
    // here - every segfaulting program died in the handler and never printed FAIL_TEST at all.
    println!("\nFAIL_TEST");
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    eprintln!("{}", Backtrace::force_capture());
    let _ = std::io::stderr().flush();
    std::process::exit(134);
}

#[cfg(windows)]
unsafe extern "system" fn crash_handler(_info: *mut std::ffi::c_void) -> i32 {
    report_segfault_and_exit();
}

#[cfg(unix)]
extern "C" fn crash_handler_unix(_sig: libc::c_int) {
    report_segfault_and_exit();
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn GC_init();
    /// `GC_REGISTER_DISPLACEMENT` in gc.h is a macro over this; only this symbol is exported.
    fn GC_register_displacement(n: usize);
}

#[ctor]
fn init() {
    // Must come first, and before any allocation in this function: the global allocator routes
    // through the collector once this latch flips, so the collector has to be usable by then.
    // `GC_REGISTER_DISPLACEMENT(8)` covers `toy_malloc_struct`, which hands the program a `base + 8`
    // body pointer — without it that interior pointer keeps the struct alive only by way of bdwgc's
    // `GC_all_interior_pointers` default.
    crate::ctla::init_toy_gc();
    #[cfg(target_os = "linux")]
    if crate::ctla::gc_enabled() {
        unsafe {
            GC_init();
            GC_register_displacement(8);
        }
    }

    #[cfg(windows)]
    unsafe {
        SetUnhandledExceptionFilter(Some(crash_handler));
    }
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGSEGV, crash_handler_unix as libc::sighandler_t);
    }

    let _ = PROGRAM_START.set(std::time::Instant::now());

    init_toy_debug();

    crate::toy_std::fuzz::init_fuzz_half_life();

    DEBUG_HEAP.set(Mutex::new(DebugHeap::new())).unwrap();

    *DEBUG_HEAP.get().unwrap().lock().unwrap() = DebugHeap::new();

    let args: Vec<std::ffi::CString> = std::env::args()
        .map(|a| std::ffi::CString::new(a).unwrap())
        .collect();

    unsafe { GLOBAL_ARGC = args.len() as i64 };
    unsafe {
        GLOBAL_ARGV = libc::malloc(std::mem::size_of::<*mut libc::c_char>() * args.len())
            as *mut *mut libc::c_char
    };

    for (i, arg) in args.iter().enumerate() {
        let bytes = arg.as_bytes_with_nul();
        let ptr = unsafe { libc::malloc(bytes.len()) as *mut libc::c_char };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const libc::c_char, ptr, bytes.len())
        };
        unsafe { *GLOBAL_ARGV.add(i) = ptr };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let res = unsafe { user_main() };
    //wtf, forget why this is here
    //for i in 0..unsafe {GLOBAL_ARGC} {
    //    unsafe { libc::free(*GLOBAL_ARGV.add(i as usize) as *mut libc::c_void) };
    //}
    //unsafe { libc::free(GLOBAL_ARGV as *mut libc::c_void) };
    //unsafe {GLOBAL_ARGV = std::ptr::null_mut()};
    let total_bytes = *TOTAL_ALLOCATION_SIZES.lock().unwrap();
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let _ = fs::create_dir_all("./temp/FUZZ_BYTES");

    let (live_allocs, malloc_calls, mut lifetimes_ns) = {
        let heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
        (
            heap.total_live_allocations,
            heap.total_allocations as u64,
            heap.lifetimes_ns.clone(),
        )
    };

    let mut stats = read_ctla_stats().expect("[ERROR] CTLA stats blob missing or malformed");
    stats.total_bytes = Some(total_bytes);
    stats.malloc_calls = Some(malloc_calls);
    if !lifetimes_ns.is_empty() {
        lifetimes_ns.sort_unstable();
        let count = lifetimes_ns.len() as u64;
        let mean = lifetimes_ns.iter().sum::<u64>() / count;
        let median = lifetimes_ns[lifetimes_ns.len() / 2];
        let min = lifetimes_ns[0];
        let max = *lifetimes_ns.last().unwrap();
        stats.lifetime_mean_ns = Some(mean);
        stats.lifetime_median_ns = Some(median);
        stats.lifetime_min_ns = Some(min);
        stats.lifetime_max_ns = Some(max);
    }
    stats.gc_is_on = Some(crate::ctla::gc_enabled());
    stats.program_runtime_ns = PROGRAM_START.get().map(|start| start.elapsed().as_nanos() as u64);
    let json = serde_json::to_string(&stats).unwrap_or_default();
    let _ = fs::write(format!("./temp/FUZZ_BYTES/{}.json", ms), json);

    // Under TOY_GC the collector owns reclamation, so CTLA's frees are advisory and a non-zero live
    // count says nothing about leaks.
    if live_allocs != 0 && !crate::ctla::gc_enabled() {
        _print_debug_heap();
        println!("\nFAIL_TEST");
        panic!();
    }

    return res as i32;
}
