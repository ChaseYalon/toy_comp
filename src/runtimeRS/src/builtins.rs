use crate::ToyPtr;
use crate::ctla::{_check_pointer, toy_free};
use crate::stub::DEBUG_HEAP;
use std::ffi::{CStr, CString};
use std::io;
use std::io::Write;
use std::collections::HashMap;
use std::os::raw::c_void;
use std::time::Instant;
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
    /// Parallel to `arr`: `owned[i]` means the array owns `arr[i]` and is responsible for freeing it
    /// (on eviction or at deep-free). A borrowed slot is owned by some independent variable and is
    /// never freed by the array. The compiler sets this at the write site; the runtime only reads it.
    owned: Vec<bool>,
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
            // When degree > 1, elements are still sub-arrays, so keep the array type.
            // When degree == 1, elements are scalars, so use the element type.
            // Struct arrays have no scalar element type; struct values format as their
            // raw pointer (matching a bare `println(some_struct)`), i.e. ToyType::Int.
            let elem_type = if degree > 1 {
                array.ty.clone()
            } else if array.ty == ToyType::Struct {
                ToyType::Int
            } else {
                array.ty.to_elem_type()
            };

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

            let quote_elems = elem_type == ToyType::Str;
            buff.push('[');
            for (i, s) in element_strs.iter().enumerate() {
                if quote_elems { buff.push('"'); }
                buff.push_str(s);
                if quote_elems { buff.push('"'); }
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
///copies a struct + an 8 byte size prefix into the heap. Returns a pointer to the struct, subtract 8 to get the prefix bytes
pub fn toy_malloc_struct(size: i64, toy_struct: ToyPtr) -> ToyPtr {
    let total = size as usize + 8; // 8 bytes prefix for size
    let out = meta_malloc!(total) as *mut u8;
    if out.is_null() {
        panic!("[ERROR] Meta malloc failed");
    }
    unsafe {
        *(out as *mut i64) = size; // write size as prefix
        libc::memcpy(out.add(8) as *mut c_void, toy_struct as *mut c_void, size as usize);
    }
    return unsafe {out.add(8)} as ToyPtr; // return pointer PAST the prefix
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

    let owned = vec![false; len as usize];
    let toy_arr = Box::new(ToyArr {
        ty: arr_type,
        degree,
        should_free_subelements: false,
        arr,
        owned,
    });

    let ptr = Box::into_raw(toy_arr) as ToyPtr;
    if let Ok(v) = std::env::var("TOY_DEBUG") {
        if v == "TRUE" {
            let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
            heap.map.insert(ptr, (std::mem::size_of::<ToyArr>() as i64, Instant::now()));
            heap.total_live_allocations += 1;
            heap.total_allocations += 1;
        }
    }
    return ptr;
}

#[unsafe(no_mangle)]
///ty refers to the type of the array, so 4 for str[] not the type of the elements
pub fn toy_write_to_arr(arr_in_ptr: ToyPtr, value: i64, idx: i64, ty: i64) {
    let _ = arr_swap_impl(arr_in_ptr, value, idx, ty, true);
}
#[unsafe(no_mangle)]
/// Like `toy_write_to_arr` but marks the slot as borrowed: the array references `value` but does not
/// own it (some independent variable does), so the array will never free it.
pub fn toy_write_to_arr_borrowed(arr_in_ptr: ToyPtr, value: i64, idx: i64, ty: i64) {
    let _ = arr_swap_impl(arr_in_ptr, value, idx, ty, false);
}
#[unsafe(no_mangle)]
/// Writes `value` into slot `idx` (marking the slot owned) and returns the previous occupant when the
/// array owned it (0 otherwise — borrowed slot, empty/freshly-grown slot, or a self-write-back).
/// Surfacing the evicted owned value lets CTLA free it at compile time instead of a runtime sweep.
/// `ty` refers to the type of the array, so 4 for str[] not the element type.
pub fn toy_arr_swap(arr_in_ptr: ToyPtr, value: i64, idx: i64, ty: i64) -> i64 {
    arr_swap_impl(arr_in_ptr, value, idx, ty, true)
}
#[unsafe(no_mangle)]
/// Like `toy_arr_swap` but marks the written slot borrowed: the incoming `value` is owned by an
/// independent variable, so the array will not free it. The evicted occupant is still returned iff
/// the array owned that previous value.
pub fn toy_arr_swap_borrowed(arr_in_ptr: ToyPtr, value: i64, idx: i64, ty: i64) -> i64 {
    arr_swap_impl(arr_in_ptr, value, idx, ty, false)
}
/// `new_owned` is the compiler-set ownership bit for the incoming `value` (true = array owns it).
fn arr_swap_impl(arr_in_ptr: ToyPtr, value: i64, idx: i64, ty: i64, new_owned: bool) -> i64 {
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
        arr_ptr.owned.resize(idx as usize + 1, false);
    }
    let degree = arr_ptr.degree;
    let elem_arr_ty = arr_ptr.ty.clone();
    let old = arr_ptr.arr[idx as usize];
    let old_owned = arr_ptr.owned[idx as usize];
    arr_ptr.arr[idx as usize] = value;
    // Self-write-back into the SAME slot (`arr[i] = arr[i]`): writing the pointer the slot already
    // holds leaves ownership unchanged. Preserve the prior ownership instead of taking `new_owned`,
    // which would let a borrowed self-write (compiler-routed for self-encapsulation) drop the slot's
    // sole owner and leak. This is the per-slot self-write-back case, not a scan over the array.
    let self_same_slot = old == value;
    arr_ptr.owned[idx as usize] = if self_same_slot { old_owned } else { new_owned };
    // The evicted value is reclaimable only if the array owned it. A borrowed slot's value is owned
    // by an independent variable and must not be freed here.
    // Self-write-back: the evicted value is the value we just stored, so it is still live in the
    // slot. Report 0 so the (null-safe) eviction free is a no-op.
    if !old_owned || self_same_slot {
        return 0;
    }
    // Borrowed write: the array gives up this slot, so no caller receives the evicted owned value to
    // free. Reclaim it here, with the element-type-correct free, instead of leaking it.
    if !new_owned {
        free_owned_arr_element(degree, &elem_arr_ty, old);
        return 0;
    }
    return old;
}
/// Frees an owned element evicted from an array, matching `toy_free_arr`'s per-element rules:
/// nested-array elements (degree > 1) deep-free; scalar str/struct elements free their pointer.
fn free_owned_arr_element(degree: i64, arr_ty: &ToyType, val: i64) {
    if val == 0 {
        return;
    }
    if degree > 1 {
        toy_deep_free_arr(val);
    } else {
        let elem_type = arr_ty.to_elem_type();
        if elem_type == ToyType::Str || elem_type == ToyType::Struct {
            toy_free(val as *mut c_void);
        }
    }
}
/// Null-safe toy_free for swap-evicted scalars: a self-write-back swap reports the evicted pointer
/// as 0, which must not be freed.
#[unsafe(no_mangle)]
pub fn toy_free_evicted(ptr: ToyPtr) {
    if ptr != 0 {
        toy_free(ptr as *mut c_void);
    }
}
/// Null-safe toy_deep_free_arr for swap-evicted nested arrays.
#[unsafe(no_mangle)]
pub fn toy_deep_free_arr_evicted(ptr: ToyPtr) {
    if ptr != 0 {
        toy_deep_free_arr(ptr);
    }
}

// --- Per-field struct ownership -------------------------------------------------------------------
// A struct is a raw heap blob of 8-byte fields with no room for metadata, so per-field ownership
// (does the struct own field i, i.e. is it responsible for freeing it) is tracked in this side
// table, keyed on the struct BODY pointer. It mirrors `ToyArr.owned` for arrays: the compiler sets a
// field's bit at each write (owned vs a borrowed param / read-out / alias), and the runtime reads it
// when a field is overwritten (eviction) or when the struct dies. This is O(1) per operation — not a
// scan/sweep. Keyed on the body pointer, which is what the program holds and what `toy_free_struct`
// clears; address reuse is safe because `toy_struct_init_owned` overwrites and free clears.
static STRUCT_FIELD_OWNED: std::sync::OnceLock<std::sync::Mutex<HashMap<i64, u64>>> =
    std::sync::OnceLock::new();
fn struct_owned_map() -> &'static std::sync::Mutex<HashMap<i64, u64>> {
    STRUCT_FIELD_OWNED.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}
/// Registers a freshly created struct's initial per-field ownership bitmap (bit i = struct owns
/// field i). Called by CTLA right after `toy_malloc_struct`.
#[unsafe(no_mangle)]
pub fn toy_struct_init_owned(body: ToyPtr, bitmap: i64) {
    struct_owned_map().lock().unwrap().insert(body, bitmap as u64);
}
/// Sets field `idx`'s ownership bit (called after a field write to record the new value's ownership).
#[unsafe(no_mangle)]
pub fn toy_struct_set_owned(body: ToyPtr, idx: i64, owned: i64) {
    let mut m = struct_owned_map().lock().unwrap();
    let bm = m.entry(body).or_insert(u64::MAX);
    if owned != 0 {
        *bm |= 1u64 << idx;
    } else {
        *bm &= !(1u64 << idx);
    }
}
/// Drops a struct's ownership entry (called by `toy_free_struct`), so a later reuse of the address
/// starts clean.
#[unsafe(no_mangle)]
pub fn toy_struct_forget(body: ToyPtr) {
    struct_owned_map().lock().unwrap().remove(&body);
}
fn struct_field_is_owned(body: ToyPtr, idx: i64) -> bool {
    match struct_owned_map().lock().unwrap().get(&body) {
        Some(bm) => (bm >> idx) & 1 == 1,
        // An uninstrumented struct (e.g. one returned from an extern module) has no entry; keep the
        // prior behavior of freeing its owned fields.
        None => true,
    }
}
/// Frees struct field `idx`'s heap value ONLY if the struct currently owns it. `ty_code`: 0 = str
/// (scalar pointer), 1 = array (deep-free), 2 = struct. Used for both the eviction of an overwritten
/// field and the reclamation of the surviving field at struct death.
#[unsafe(no_mangle)]
pub fn toy_struct_free_field_if_owned(body: ToyPtr, idx: i64, value: i64, ty_code: i64) {
    if value == 0 || !struct_field_is_owned(body, idx) {
        return;
    }
    match ty_code {
        1 => toy_deep_free_arr(value),
        2 => crate::ctla::toy_free_struct(value),
        _ => toy_free(value as *mut c_void),
    }
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
/// Marks every slot currently holding `value` as borrowed: ownership of that element was
/// transferred elsewhere (e.g. read out of this array and owned-written into a parameter array),
/// so this array's deep-free must skip it. Slots still owned (the unpicked elements) are
/// unaffected and get reclaimed by the deep-free. Only the runtime knows which slot the dynamic
/// read index picked, which is why this cannot be resolved at compile time.
pub fn toy_arr_disown(arr_in_ptr: ToyPtr, value: i64) {
    _check_pointer(arr_in_ptr as *mut c_void);
    let arr_ptr = unsafe { &mut *(arr_in_ptr as *mut ToyArr) };
    for (i, &v) in arr_ptr.arr.iter().enumerate() {
        if v == value {
            arr_ptr.owned[i] = false;
        }
    }
}
#[unsafe(no_mangle)]
pub fn toy_free_arr(arr_ptr_int: ToyPtr) {
    _check_pointer(arr_ptr_int as *mut c_void);
    let arr = unsafe { &mut *(arr_ptr_int as *mut ToyArr) };

    if arr.should_free_subelements {
        // Ownership is disjoint by construction (the compiler marks duplicates/read-backs borrowed,
        // so each allocation has at most one owned slot), so each owned slot is freed exactly once
        // with no runtime dedup. Borrowed slots are freed by their independent owner.
        if arr.degree > 1 {
            // Elements are nested arrays — recursively free the owned ones
            for (i, &val) in arr.arr.iter().enumerate() {
                if val != 0 && arr.owned[i] {
                    toy_deep_free_arr(val);
                }
            }
        } else {
            // Elements are scalars — only free owned heap-allocated types
            let elem_type = arr.ty.to_elem_type();
            for (i, &val) in arr.arr.iter().enumerate() {
                if (elem_type == ToyType::Str || elem_type == ToyType::Struct)
                    && val != 0
                    && arr.owned[i]
                {
                    toy_free(val as *mut c_void);
                }
            }
        }
    }

    if let Ok(v) = std::env::var("TOY_DEBUG") {
        if v == "TRUE" {
            let mut heap = DEBUG_HEAP.get().unwrap().lock().unwrap();
            if let Some(&(size, alloc_time)) = heap.map.get(&arr_ptr_int) {
                if size != -1 {
                    heap.total_live_allocations -= 1;
                    heap.lifetimes_ns.push(alloc_time.elapsed().as_nanos() as u64);
                }
            }
            heap.map.insert(arr_ptr_int, (-1, Instant::now()));
        }
    }

    unsafe { drop(Box::from_raw(arr_ptr_int as *mut ToyArr)) };
}
#[unsafe(no_mangle)]
pub fn toy_deep_free_arr(arr_ptr_int: ToyPtr) {
    _check_pointer(arr_ptr_int as *mut c_void);
    let arr = unsafe { &mut *(arr_ptr_int as *mut ToyArr) };
    arr.should_free_subelements = true;
    toy_free_arr(arr_ptr_int);
}
#[unsafe(no_mangle)]
pub fn toy_arr_concat(arr1: ToyPtr, arr2: ToyPtr) -> ToyPtr {
    _check_pointer(arr1 as *mut c_void);
    _check_pointer(arr2 as *mut c_void);
    let a1 = unsafe { &mut *(arr1 as *mut ToyArr) };
    let a2 = unsafe { &mut *(arr2 as *mut ToyArr) };

    let total_len = a1.arr.len() + a2.arr.len();
    let res_ptr = toy_malloc_arr(total_len as i64, a1.ty.clone() as i64, a1.degree);
    let res = unsafe { &mut *(res_ptr as *mut ToyArr) };

    res.arr.extend_from_slice(&a1.arr);
    res.arr.extend_from_slice(&a2.arr);
    // Keep `owned` parallel to `arr` and move ownership of the elements into the result so no
    // allocation is owned by two arrays: the result owns whatever the sources owned, and the source
    // slots become borrowed.
    res.owned.extend_from_slice(&a1.owned);
    res.owned.extend_from_slice(&a2.owned);
    for o in a1.owned.iter_mut() {
        *o = false;
    }
    for o in a2.owned.iter_mut() {
        *o = false;
    }

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

#[unsafe(no_mangle)]
///takes the memory at buffer to_copy (assumes it is valid) and allocates a new version in the heap, then copies byte for byte
///_degree is not read, it only exits because inject_type_params requires a degree and I dont want to write a new one
pub fn toy_mem_dup(to_copy: ToyPtr, ty: i64, _degree: i64) ->ToyPtr{
    let toy_type = ToyType::try_from(ty).unwrap();
    if toy_type == ToyType::Struct {
        // Structs are allocated with an 8-byte size prefix by toy_malloc_struct.
        // Duplicate the full allocation (prefix + body) and return pointer past the prefix.
        let size = unsafe { *((to_copy as *const u8).sub(8) as *const i64) };
        let total = size as usize + 8;
        let out = meta_malloc!(total) as *mut u8;
        unsafe {
            *(out as *mut i64) = size;
            libc::memcpy(out.add(8) as *mut c_void, to_copy as *const c_void, size as usize);
        }
        return unsafe { out.add(8) } as ToyPtr;
    }
    // Strings need the null terminator included in the copy
    if toy_type == ToyType::Str {
        let len = toy_strlen(to_copy) as usize;
        let new_buff = meta_malloc!(len + 1);
        unsafe { libc::memcpy(new_buff, to_copy as *const c_void, len + 1) };
        return new_buff as ToyPtr;
    }
    let len: u64 = match toy_type {
        ToyType::BoolArr
        | ToyType::FloatArr
        | ToyType::IntArr
        | ToyType::StrArr => toy_arrlen(to_copy) as u64,
        _ => unreachable!()
    };
    let new_buff = meta_malloc!(len as usize);
    unsafe {libc::memcpy(new_buff, to_copy as *const c_void, len as usize)};
    return new_buff as ToyPtr;
}