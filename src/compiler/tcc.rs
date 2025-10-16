use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::fs;
use std::io::Write;
use std::path::Path;

#[repr(C)]
pub struct TCCState {
    _private: [u8; 0],
}

#[link(name = "tcc", kind = "static")]
unsafe extern "C" {
    pub fn tcc_new() -> *mut TCCState;
    pub fn tcc_delete(s: *mut TCCState);
    pub fn tcc_set_output_type(s: *mut TCCState, output_type: c_int) -> c_int;
    pub fn tcc_compile_string(s: *mut TCCState, code: *const c_char) -> c_int;
    pub fn tcc_add_file(s: *mut TCCState, filename: *const c_char) -> c_int;
    pub fn tcc_output_file(s: *mut TCCState, filename: *const c_char) -> c_int;
}

pub const TCC_OUTPUT_EXE: c_int = 2;

unsafe fn add_file(state: *mut TCCState, path: &str) {
    let c_path = CString::new(path).unwrap();
    if unsafe { tcc_add_file(state, c_path.as_ptr()) } != 0 {
        panic!("failed to add file {path}");
    }
}

fn compile_string(state: *mut TCCState, src: &str) {
    let csrc = CString::new(src).unwrap();
    if unsafe { tcc_compile_string(state, csrc.as_ptr()) } != 0 {
        panic!("failed to compile C stub");
    }
}

fn write_exe(state: *mut TCCState, path: &str) {
    let c_path = CString::new(path).unwrap();
    unsafe { tcc_set_output_type(state, TCC_OUTPUT_EXE) };
    if unsafe { tcc_output_file(state, c_path.as_ptr()) } != 0 {
        panic!("failed to write exe");
    }
}

pub fn link_and_write_exe(obj_bytes: &[u8], exe_path: &str) {
    unsafe {
        let state = tcc_new();
        assert!(!state.is_null(), "failed to init TCC");

        let tmp_obj_path = Path::new("tmp_obj.o");
        fs::File::create(&tmp_obj_path)
            .unwrap()
            .write_all(obj_bytes)
            .unwrap();

        add_file(state, tmp_obj_path.to_str().unwrap());

        //No point giving it a file
        let stub = r#"
            extern long user_main();
            int main() { return (int)user_main(); }
        "#;
        compile_string(state, stub);

        write_exe(state, exe_path);

        tcc_delete(state);
        let _ = fs::remove_file(tmp_obj_path);
    }
}