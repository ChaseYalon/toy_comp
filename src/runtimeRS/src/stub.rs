use crate::{TOTAL_ALLOCATION_SIZES, ctla::{_print_debug_heap, DebugHeap}};
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
    escape_prog_pct: Option<f64>,
    total_bytes: Option<u64>,
    malloc_calls: Option<u64>,
    lifetime_mean_ns: Option<u64>,
    lifetime_median_ns: Option<u64>,
    lifetime_min_ns: Option<u64>,
    lifetime_max_ns: Option<u64>,
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

#[unsafe(no_mangle)]
pub static mut GLOBAL_ARGC: i64 = 0;

#[unsafe(no_mangle)]
pub static mut GLOBAL_ARGV: *mut *mut libc::c_char = std::ptr::null_mut();

unsafe extern "system" {
    fn SetUnhandledExceptionFilter(
        filter: Option<unsafe extern "system" fn(*mut std::ffi::c_void) -> i32>,
    ) -> *mut std::ffi::c_void;
}

unsafe extern "system" fn crash_handler(_info: *mut std::ffi::c_void) -> i32 {
    use std::io::Write;
    eprintln!("\n[FATAL] toy_lang runtime aborted: memory access violation (segfault)");
    let bt = Backtrace::force_capture();
    eprintln!("{}", bt);
    println!("\nFAIL_TEST");
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(134);
}

#[ctor]
fn init() {
    unsafe { SetUnhandledExceptionFilter(Some(crash_handler)) };

    DEBUG_HEAP.set(Mutex::new(DebugHeap::new())).unwrap();

    *DEBUG_HEAP.get().unwrap().lock().unwrap() = DebugHeap::new();

    unsafe { std::env::set_var("TOY_DEBUG", "TRUE") };

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
    for i in 0..unsafe {GLOBAL_ARGC} {
        unsafe { libc::free(*GLOBAL_ARGV.add(i as usize) as *mut libc::c_void) };
    }
    unsafe { libc::free(GLOBAL_ARGV as *mut libc::c_void) };
    unsafe {GLOBAL_ARGV = std::ptr::null_mut()};
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
    let json = serde_json::to_string(&stats).unwrap_or_default();
    let _ = fs::write(format!("./temp/FUZZ_BYTES/{}.json", ms), json);

    if live_allocs != 0 {
        _print_debug_heap();
        println!("\nFAIL_TST");
        panic!();
    }

    return res as i32;
}
