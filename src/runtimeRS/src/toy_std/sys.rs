use crate::ToyPtr;
use crate::builtins;
use crate::stub::{GLOBAL_ARGC, GLOBAL_ARGV};
use std::ffi::CStr;
use std::process;
use std::process::Command;
#[unsafe(no_mangle)]
fn toy_sys_exit(code: i64) {
    process::exit(code as i32);
}
#[unsafe(no_mangle)]
fn toy_sys_get_pid() -> i64 {
    return process::id() as i64;
}

#[unsafe(no_mangle)]
fn toy_sys_get_argc() -> i64 {
    return unsafe { GLOBAL_ARGC };
}
#[unsafe(no_mangle)]
fn toy_sys_get_argv() -> ToyPtr {
    let arr = builtins::toy_malloc_arr(toy_sys_get_argc(), 4, 1);
    for i in 0..toy_sys_get_argc() {
        let val = unsafe { *GLOBAL_ARGV.offset(i as isize) };
        builtins::toy_write_to_arr(arr, val as i64, i, 4);
    }
    return arr;
}

#[unsafe(no_mangle)]
fn toy_sys_get_os_name() -> i64 {
    let s = if cfg!(windows) {
        "windows\0"
    } else {
        "linux\0"
    };
    return builtins::toy_malloc(s.as_ptr() as i64);
}

#[unsafe(no_mangle)]
fn toy_sys_get_core_count() -> i64 {
    return num_cpus::get() as i64;
}

#[unsafe(no_mangle)]
fn toy_sys_is_little_endian() -> i64 {
    return if cfg!(target_endian = "little") { 1 } else { 0 };
}

#[unsafe(no_mangle)]
fn toy_sys_invoke(code: ToyPtr, args: ToyPtr) -> i64 {
    let name = unsafe { CStr::from_ptr(code as *const i8).to_str().unwrap() };
    let mut rust_args: Vec<&str> = vec![];
    let arg_count = builtins::toy_arrlen(args);
    for i in 0..arg_count {
        let s = unsafe {
            CStr::from_ptr(builtins::toy_read_from_arr(args, i) as *const i8)
                .to_str()
                .unwrap()
        };
        rust_args.push(s);
    }
    let child = Command::new(name).args(rust_args).spawn();

    let mut child = match child {
        Ok(child) => child,
        Err(_) => return -1,
    };

    let status = match child.wait() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match status.code() {
        Some(code) => code as i64,
        None => -2,
    }
}
