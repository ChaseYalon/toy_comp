use crate::ToyPtr;
use crate::ctla::{_check_pointer, toy_free};
use crate::stub::DEBUG_HEAP;
use std::ffi::{CStr, CString};
use std::io;
use std::io::Write;
use std::os::raw::c_void;
use crate::values::ToyType;
//datatype is 0 for string, 1 for bool, 2 for int, 3 for float, 4 for str[], 5 for bool[], 6 for int[], 7 for float[], 8 for struct[]
//if datatype is 0 (input is string) then input is a pointer
//Input could be an int, if sizeof(type) > wordSize
#[repr(C)]
//it is ub to reference a ToyArr from C.
pub struct ToyArr {
    ty: ToyType,
    degree: i64,
    pub should_free_subelements: bool,
    arr: Vec<i64>,
}
#[macro_export]
macro_rules! meta_malloc {
    ($size:expr) => {{
        match ::std::env::var("TOY_DEBUG").as_deref() {
            Ok("TRUE") => $crate::ctla::_toy_malloc_debug($size),
            _ => unsafe { ::libc::malloc($size) },
        }
    }};
}
#[unsafe(no_mangle)]
pub fn _toy_format(input: ToyPtr, datatype: ToyType, degree: i64) -> *mut i8 {
    match datatype {
        ToyType::Str => {
            if input == 0 {
                return CString::new("NULL_STRING").unwrap().into_raw();
            }
            let str = unsafe { CStr::from_ptr(input as *const i8) };
            return str.to_owned().into_raw();
        }
        ToyType::Bool => {
            if input == 1 {
                return CString::new("true").unwrap().into_raw();
            } else if input == 0 {
                return CString::new("false").unwrap().into_raw();
            } else {
                panic!("[ERROR] Expected boolean but found {input}");
            }
        }
        ToyType::Int => {
            return CString::new(format!("{input}")).unwrap().into_raw();
        }
        ToyType::Float => {
            let f = f64::from_bits(input as u64);
            return CString::new(format!("{f}")).unwrap().into_raw();
        }
        _ => {
            let array = unsafe { &*(input as *const ToyArr) };
            let elem_type = array.ty.to_elem_type(); // convert array type to element type

            let mut element_strs: Vec<String> = Vec::with_capacity(array.arr.len());

            for &val in &array.arr {
                let raw = _toy_format(val, elem_type.clone(), degree - 1);
                let s = unsafe { CString::from_raw(raw) }
                    .to_string_lossy()
                    .into_owned();
                element_strs.push(s);
            }

            let mut buff = String::with_capacity(
                2 + element_strs.iter().map(|s| s.len()).sum::<usize>()
                    + if array.arr.len() > 1 {
                        (array.arr.len() - 1) * 2
                    } else {
                        0
                    },
            );

            buff.push('[');
            for (i, s) in element_strs.iter().enumerate() {
                buff.push_str(s);
                if i != element_strs.len() - 1 {
                    buff.push_str(", ");
                }
            }
            buff.push(']');

            return CString::new(buff).unwrap().into_raw();
        }
    }
}
#[unsafe(no_mangle)]
pub fn toy_print(input: ToyPtr, datatype: i64, degree: i64) {
    let raw = _toy_format(input, ToyType::try_from(datatype).unwrap(), degree);
    let s = unsafe { CStr::from_ptr(raw).to_str().unwrap() };
    print!("{}", s);
    std::io::stdout().flush().unwrap(); //this seems wrong...
    unsafe {
        drop(CString::from_raw(raw));
    }
}

#[unsafe(no_mangle)]
pub fn toy_println(input: ToyPtr, datatype: i64, degree: i64) {
    let raw = _toy_format(input, ToyType::try_from(datatype).unwrap(), degree);
    let s = unsafe { CStr::from_ptr(raw).to_str().unwrap() };
    println!("{}", s);
    unsafe { drop(CString::from_raw(raw)) };
}

#[unsafe(no_mangle)]
pub fn toy_malloc(ptr: ToyPtr) -> ToyPtr {
    //causes bizarre bug where test fails in a group but not on its own: _check_pointer(ptr as *mut c_void);
    let input = unsafe { CStr::from_ptr(ptr as *const i8) };
    let bytes = input.to_bytes_with_nul(); // includes null terminator
    let out = meta_malloc!(bytes.len()) as *mut u8;
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
    return out as ToyPtr;
}
#[unsafe(no_mangle)]
pub fn toy_concat(sp1: ToyPtr, sp2: ToyPtr) -> ToyPtr {
    _check_pointer(sp1 as *mut c_void);
    _check_pointer(sp2 as *mut c_void);
    let str1 = unsafe { CStr::from_ptr(sp1 as *const i8) };
    let str2 = unsafe { CStr::from_ptr(sp2 as *const i8) };
    let b1 = str1.to_bytes();
    let b2 = str2.to_bytes_with_nul();
    let out = meta_malloc!(b1.len() + b2.len()) as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(b1.as_ptr(), out, b1.len());
        std::ptr::copy_nonoverlapping(b2.as_ptr(), out.add(b1.len()), b2.len());
    }
    return out as ToyPtr;
}
//the fact that this function exists is a failure in design
#[unsafe(no_mangle)]
pub fn toy_str_arr_to_str(arr: ToyPtr) -> ToyPtr {
    _check_pointer(arr as *mut c_void);
    let tmp = _toy_format(arr, ToyType::StrArr, 1);
    let out = toy_malloc(tmp as ToyPtr);
    unsafe { drop(CString::from_raw(tmp)) };
    return out;
}

#[unsafe(no_mangle)]
/// 1 if true, 0 if false
pub fn toy_strequal(sp1: ToyPtr, sp2: ToyPtr) -> i64 {
    _check_pointer(sp1 as *mut c_void);
    _check_pointer(sp2 as *mut c_void);
    let str1 = unsafe { CStr::from_ptr(sp1 as *const i8) };
    let str2 = unsafe { CStr::from_ptr(sp2 as *const i8) };
    if str1 == str2 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub fn toy_strlen(sp1: ToyPtr) -> i64 {
    _check_pointer(sp1 as *mut c_void);
    return unsafe { libc::strlen(sp1 as *mut i8) } as i64;
}

#[unsafe(no_mangle)]
pub fn toy_type_to_str(val: i64, ty: i64) -> ToyPtr {
    return _toy_format(val,ToyType::try_from(ty).unwrap(), 1) as i64;
}

#[unsafe(no_mangle)]
pub fn toy_type_to_bool(val: i64, ty: i64) -> i64 {
    let toy_ty = ToyType::try_from(ty).unwrap();
    if toy_ty == ToyType::Str{
        if toy_strequal(val, c"true".as_ptr() as i64) == 1 {
            return 1;
        }
        if toy_strequal(val, c"false".as_ptr() as i64) == 1 {
            return 0;
        }
    }
    if toy_ty == ToyType::Bool {
        return val;
    }
    if toy_ty == ToyType::Int {
        if val >= 0 && val <= 1 {
            return val;
        }
        panic!(
            "[ERROR] Tried to convert integer {val} to bool. Only 1 and 0 maybe be converted to bools"
        );
    }
    if toy_ty == ToyType::Float {
        let f = f64::from_bits(val as u64);
        return if f < 0.0 { 0 } else { 1 };
    }
    panic!("[ERROR] Runtime cannot convert type {ty} to a bool.");
}

#[unsafe(no_mangle)]
pub fn toy_type_to_int(val: i64, ty: i64) -> i64 {
    let toy_ty = ToyType::try_from(ty).unwrap();
    if toy_ty == ToyType::Str {
        return unsafe { CStr::from_ptr(val as *mut i8) }
            .to_str()
            .unwrap()
            .parse()
            .expect("[ERROR] String contains non-numeric elements");
    }
    if toy_ty == ToyType::Bool {
        if val >= 0 && val <= 1 {
            return val;
        }
        panic!("[ERROR] Tried to convert {val} as a boolean to int. {val} is not a boolean");
    }
    if toy_ty == ToyType::Int {
        return val;
    }
    if toy_ty == ToyType::Float {
        let f = f64::from_bits(val as u64);
        return f.round() as i64;
    }
    panic!("[ERROR] Runtime cannot convert type {ty} to an int");
}

#[unsafe(no_mangle)]
pub fn toy_type_to_float(val: i64, ty: i64) -> i64 {
    let toy_ty = ToyType::try_from(ty).unwrap();
    if toy_ty == ToyType::Str {
        let f: f64 = unsafe { CStr::from_ptr(val as *mut i8) }
            .to_str()
            .unwrap()
            .parse()
            .expect("[ERROR] string contains non-numeric elements");
        return f.to_bits() as i64;
    }
    if toy_ty == ToyType::Bool {
        if val == 0 {
            return 0.0f64.to_bits() as i64;
        }
        if val == 1 {
            return 1.0f64.to_bits() as i64;
        }
        panic!("[ERROR] Tried to convert {val} from a boolean to a float. {val} is not a boolean.");
    }
    if toy_ty == ToyType::Int {
        //this is a breaking change from the CRuntime. It made no sense to have this be a raw bitcast, so it is a type promotoion instead.
        return (val as f64).to_bits() as i64;
    }
    if toy_ty == ToyType::Float {
        return val;
    }
    panic!("[ERROR] Runtime cannot convert {ty} to an int");
}

#[unsafe(no_mangle)]
pub fn toy_int_to_float(i: i64) -> f64 {
    return i as f64;
}

#[unsafe(no_mangle)]
pub fn toy_float_bits_to_double(f_bits: i64) -> f64 {
    return f64::from_bits(f_bits as u64);
}

#[unsafe(no_mangle)]
pub fn toy_double_to_float_bits(d: f64) -> i64 {
    return d.to_bits() as i64;
}

#[unsafe(no_mangle)]
///Takes a pointer and its size in BYTES and copies that pointer to the heap.
pub fn toy_malloc_struct(size: i64, toy_struct: ToyPtr) -> ToyPtr {
    let out = meta_malloc!(size as usize);
    if out.is_null() {
        panic!("[ERROR] Meta malloc failed");
    }
    unsafe { libc::memcpy(out, toy_struct as *mut c_void, size as usize) };
    return out as i64;
}
#[unsafe(no_mangle)]
pub fn toy_malloc_arr(len: i64, ty: i64, degree: i64) -> ToyPtr {
    let toy_ty = ToyType::try_from(ty).unwrap();
    let capacity = (len as f64 * 1.4) as usize;
    let mut arr = vec![0i64; len as usize];
    arr.reserve(capacity - len as usize);

    let arr_type = match toy_ty {
        ToyType::Str => ToyType::StrArr,
        ToyType::Bool => ToyType::BoolArr,
        ToyType::Int => ToyType::IntArr,
        ToyType::Float => ToyType::FloatArr,
        _ => toy_ty,
    };

    let toy_arr = Box::new(ToyArr {
        ty: arr_type,
        degree,
        should_free_subelements: false,
        arr,
    });

    return Box::into_raw(toy_arr) as ToyPtr;
}

#[unsafe(no_mangle)]
///ty refers to the type of the array, so 4 for str[] not the type of the elements
pub fn toy_write_to_arr(arr_in_ptr: ToyPtr, value: i64, idx: i64, ty: i64) {
    _check_pointer(arr_in_ptr as *mut c_void);
    let arr_ptr = unsafe { &mut *(arr_in_ptr as *mut ToyArr) };
    let toy_ty = ToyType::try_from(arr_ptr.ty.clone()).unwrap();
    if idx < 0 {
        panic!("[ERROR] Index {idx} is not above zero");
    }
    if arr_ptr.ty != toy_ty && !(arr_ptr.ty == toy_ty.to_arr_type() || arr_ptr.ty == ToyType::Struct) {
        panic!(
            "[ERROR] Was expecting type {:?}, but got type {}",
            arr_ptr.ty, ty
        );
    }
    if toy_ty == ToyType::Str {
        _check_pointer(value as *mut c_void);
    }
    if idx as usize >= arr_ptr.arr.len() {
        arr_ptr.arr.resize(idx as usize + 1, 0);
    }
    arr_ptr.arr[idx as usize] = value;
}
#[unsafe(no_mangle)]
pub fn toy_read_from_arr(arr_in_ptr: ToyPtr, idx: i64) -> i64 {
    _check_pointer(arr_in_ptr as *mut c_void);
    let arr_ptr = unsafe { &mut *(arr_in_ptr as *mut ToyArr) };
    return arr_ptr.arr[idx as usize];
}
#[unsafe(no_mangle)]
pub fn toy_arrlen(arr_in_ptr: ToyPtr) -> i64 {
    _check_pointer(arr_in_ptr as *mut c_void);
    let arr_ptr = unsafe { &mut *(arr_in_ptr as *mut ToyArr) };
    return arr_ptr.arr.len() as i64;
}
#[unsafe(no_mangle)]
pub fn toy_free_arr(arr_ptr_int: ToyPtr) {
    _check_pointer(arr_ptr_int as *mut c_void);
    let arr = unsafe { &mut *(arr_ptr_int as *mut ToyArr) };

    if arr.should_free_subelements {
        for &val in &arr.arr {
            let elem_type = arr.ty.clone();
            if elem_type.is_arr_type() {
                toy_free_arr(val);
            } else if elem_type == ToyType::Str || elem_type == ToyType::Struct {
                toy_free(val as *mut c_void);
            }
        }
    }
    //this is a bodge
    if let Some(&v) = DEBUG_HEAP
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .map
        .get(&arr_ptr_int)
    {
        if v != -1 {
            DEBUG_HEAP
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .total_live_allocations -= 1;
        }
    }
    DEBUG_HEAP
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .map
        .insert(arr_ptr_int, -1);
    unsafe { drop(Box::from_raw(arr_ptr_int as *mut ToyArr)) };
}
#[unsafe(no_mangle)]
pub fn toy_arr_concat(arr1: ToyPtr, arr2: ToyPtr) -> ToyPtr {
    _check_pointer(arr1 as *mut c_void);
    _check_pointer(arr2 as *mut c_void);
    let a1 = unsafe { &*(arr1 as *const ToyArr) };
    let a2 = unsafe { &*(arr2 as *const ToyArr) };

    let total_len = a1.arr.len() + a2.arr.len();
    let res_ptr = toy_malloc_arr(total_len as i64, a1.ty.clone() as i64, a1.degree);
    let res = unsafe { &mut *(res_ptr as *mut ToyArr) };

    res.arr.extend_from_slice(&a1.arr);
    res.arr.extend_from_slice(&a2.arr);

    return res_ptr;
}

#[unsafe(no_mangle)]
pub fn toy_input(i_prompt: ToyPtr) -> ToyPtr {
    let prompt = unsafe { CStr::from_ptr(i_prompt as *const i8) };
    print!("{}", prompt.to_str().unwrap());
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    let null_term_string = CString::new(input).unwrap();
    let out = toy_malloc(null_term_string.as_ptr() as i64);
    return out;
}
