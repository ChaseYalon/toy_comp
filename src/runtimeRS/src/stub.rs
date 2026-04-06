use crate::ctla::{DebugHeap_create, DebugHeap_free, _PrintDebug_heap, DebugHeap};
use ctor::ctor;

unsafe extern "C" {
    fn user_main() -> i64;
}

#[unsafe(no_mangle)]
pub static mut DEBUG_HEAP: *mut DebugHeap = std::ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut GLOBAL_ARGC: i64 = 0;

#[unsafe(no_mangle)]
pub static mut GLOBAL_ARGV: *mut *mut libc::c_char = std::ptr::null_mut();

#[ctor]
fn init() {
    unsafe {DEBUG_HEAP = DebugHeap_create()};

    unsafe {std::env::set_var("TOY_DEBUG", "TRUE")};

    let args: Vec<std::ffi::CString> = std::env::args()
        .map(|a| std::ffi::CString::new(a).unwrap())
        .collect();

    unsafe {GLOBAL_ARGC = args.len() as i64};
    unsafe {GLOBAL_ARGV = libc::malloc(std::mem::size_of::<*mut libc::c_char>() * args.len()) as *mut *mut libc::c_char};
    
    for (i, arg) in args.iter().enumerate() {
        let bytes = arg.as_bytes_with_nul();
        let ptr = unsafe {libc::malloc(bytes.len()) as *mut libc::c_char};
        unsafe {std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const libc::c_char, ptr, bytes.len())};
        unsafe {*GLOBAL_ARGV.add(i) = ptr};
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let res = unsafe { user_main() };

    unsafe {
        for i in 0..GLOBAL_ARGC {
            libc::free(*GLOBAL_ARGV.add(i as usize) as *mut libc::c_void);
        }
        libc::free(GLOBAL_ARGV as *mut libc::c_void);
        GLOBAL_ARGV = std::ptr::null_mut();

        if !DEBUG_HEAP.is_null() && (*DEBUG_HEAP).TotalLiveAllocations != 0 {
            _PrintDebug_heap(DEBUG_HEAP);
            println!("\nFAIL_TST");
        }
        DebugHeap_free(DEBUG_HEAP);
        DEBUG_HEAP = std::ptr::null_mut();
    }

    res as i32
}