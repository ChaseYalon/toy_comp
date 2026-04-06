use crate::ToyPtr;
use crate::builtins;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;
#[unsafe(no_mangle)]
pub fn toy_fs_read_file(toy_path: ToyPtr) -> ToyPtr {
    let ptr_path = toy_path as *mut c_char;
    let path: &str = unsafe { CStr::from_ptr(ptr_path as *const i8).to_str().unwrap() };
    let contents = fs::read_to_string(path).unwrap(); //ffi error handling sucks

    let c_string = CString::new(contents).unwrap();
    let ptr: *const i8 = c_string.as_ptr();
    let toy_ptr = builtins::toy_malloc(ptr as i64);
    return toy_ptr as ToyPtr;
}

#[unsafe(no_mangle)]
pub fn toy_fs_write_file(path: ToyPtr, content: ToyPtr) -> i64 {
    let path = unsafe { CStr::from_ptr(path as *const i8).to_str().unwrap() };
    let contents = unsafe { CStr::from_ptr(content as *mut i8).to_str().unwrap() };
    return match fs::write(path, contents) {
        Ok(_) => 0,
        Err(_) => 1,
    };
}
#[unsafe(no_mangle)]
pub fn toy_fs_append_file(path: ToyPtr, content: ToyPtr) -> i64 {
    let path = unsafe { CStr::from_ptr(path as *const i8).to_str().unwrap() };
    let contents = unsafe { CStr::from_ptr(content as *mut i8).to_str().unwrap() };
    let to_write = format!(
        "{}{}",
        contents,
        fs::read_to_string(path).unwrap().to_string()
    );
    return match fs::write(path, to_write) {
        Ok(_) => 0,
        Err(_) => 1,
    };
}

#[unsafe(no_mangle)]
pub fn toy_fs_file_size(toy_path: ToyPtr) -> i64 {
    let path = unsafe { CStr::from_ptr(toy_path as *const i8).to_str().unwrap() };
    let meta = fs::metadata(path).unwrap();
    return meta.len() as i64;
}

#[unsafe(no_mangle)]
pub fn toy_fs_get_file_count_in_dir(toy_path: ToyPtr) -> i64 {
    let path = unsafe { CStr::from_ptr(toy_path as *const i8).to_str().unwrap() };
    let count = fs::read_dir(path).unwrap().count();
    return count as i64;
}

#[unsafe(no_mangle)]
pub fn toy_fs_read_dir(toy_path: ToyPtr) -> ToyPtr {
    let path = unsafe { CStr::from_ptr(toy_path as *const i8).to_str().unwrap() };

    let entries: Vec<std::fs::DirEntry> =
        fs::read_dir(path).unwrap().filter_map(|e| e.ok()).collect();

    let files: Vec<String> = entries
        .iter()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let dirs: Vec<String> = entries
        .iter()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    unsafe {
        let toy_files = builtins::toy_malloc_arr(files.len() as i64, 4, 1);
        let toy_folders = builtins::toy_malloc_arr(dirs.len() as i64, 4, 1);

        (*(toy_files as *mut builtins::ToyArr)).should_free_subelements = false;
        (*(toy_folders as *mut builtins::ToyArr)).should_free_subelements = false;

        for (i, file) in files.iter().enumerate() {
            let c = CString::new(file.as_str()).unwrap();
            let ptr = builtins::toy_malloc(c.as_ptr() as i64);
            builtins::toy_write_to_arr(toy_files, ptr, i as i64, 0);
        }

        for (i, dir) in dirs.iter().enumerate() {
            let c = CString::new(dir.as_str()).unwrap();
            let ptr = builtins::toy_malloc(c.as_ptr() as i64);
            builtins::toy_write_to_arr(toy_folders, ptr, i as i64, 0);
        }

        let arr = builtins::toy_malloc_arr(2, 4, 2);
        (*(arr as *mut builtins::ToyArr)).should_free_subelements = false;

        builtins::toy_write_to_arr(arr, toy_files, 0, 4);
        builtins::toy_write_to_arr(arr, toy_folders, 1, 4);

        return arr as ToyPtr;
    }
}
