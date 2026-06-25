use std::os::raw::{c_char, c_double, c_int, c_long, c_void};

extern "C" {
    fn __entry() -> c_long;
}

// =============================================================================
// ESTRUCTURAS DE DATOS
// =============================================================================

/// Vector dinámico de HULK
#[repr(C)]
pub struct HulkVector {
    pub size: c_long,
    pub capacity: c_long,
    pub data: *mut *mut c_void,
}

/// Rango iterable de HULK
#[repr(C)]
pub struct Range {
    pub min: c_double,
    pub max: c_double,
    pub current: c_double,
}

// =============================================================================
// FUNCIONES DE STRING
// =============================================================================

#[no_mangle]
pub extern "C" fn hulk_concat(s1: *const c_char, s2: *const c_char) -> *const c_char {
    unsafe {
        let len1 = libc::strlen(s1);
        let len2 = libc::strlen(s2);
        let ptr = libc::malloc(len1 + len2 + 1) as *mut c_char;
        libc::sprintf(ptr, b"%s%s\0".as_ptr() as *const c_char, s1, s2);
        ptr
    }
}

#[no_mangle]
pub extern "C" fn hulk_concat_space(s1: *const c_char, s2: *const c_char) -> *const c_char {
    unsafe {
        let len1 = libc::strlen(s1);
        let len2 = libc::strlen(s2);
        let ptr = libc::malloc(len1 + len2 + 2) as *mut c_char;
        libc::sprintf(ptr, b"%s %s\0".as_ptr() as *const c_char, s1, s2);
        ptr
    }
}

#[no_mangle]
pub extern "C" fn hulk_num_to_str(num: c_double) -> *const c_char {
    unsafe {
        let ptr = libc::malloc(32) as *mut c_char;
        libc::sprintf(ptr, b"%g\0".as_ptr() as *const c_char, num);
        ptr
    }
}

// =============================================================================
// FUNCIONES DE PRINT
// =============================================================================

#[no_mangle]
pub extern "C" fn print_number(value: c_double) -> c_double {
    unsafe {
        libc::printf(b"%g\n\0".as_ptr() as *const c_char, value);
    }
    value
}

#[no_mangle]
pub extern "C" fn print_string(value: *const c_char) -> *const c_char {
    unsafe {
        libc::printf(b"%s\n\0".as_ptr() as *const c_char, value);
    }
    value
}

#[no_mangle]
pub extern "C" fn print_bool(value: bool) -> bool {
    unsafe {
        let s = if value {
            b"true\n\0".as_ptr()
        } else {
            b"false\n\0".as_ptr()
        };
        libc::printf(s as *const c_char);
    }
    value
}

#[no_mangle]
pub extern "C" fn print_object(value: *const c_char) -> *const c_char {
    unsafe {
        libc::printf(b"%s\n\0".as_ptr() as *const c_char, value);
    }
    value
}

// =============================================================================
// MEMORY MANAGEMENT
// =============================================================================

#[no_mangle]
pub extern "C" fn hulk_alloc(n: c_long) -> *mut c_void {
    if n <= 0 {
        return std::ptr::null_mut();
    }
    unsafe { libc::malloc(n as usize) }
}

#[no_mangle]
pub extern "C" fn hulk_free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    unsafe { libc::free(p) }
}

#[no_mangle]
pub extern "C" fn hulk_strlen(p: *const c_char) -> c_long {
    if p.is_null() {
        return 0;
    }
    unsafe { libc::strlen(p) as c_long }
}

// =============================================================================
// RANGE (ITERABLE PROTOCOL)
// =============================================================================

#[no_mangle]
pub extern "C" fn range(lo: c_double, hi: c_double) -> *mut c_void {
    let r = unsafe { libc::malloc(std::mem::size_of::<Range>()) as *mut Range };
    unsafe {
        (*r).min = lo;
        (*r).max = hi;
        (*r).current = lo - 1.0;
    }
    r as *mut c_void
}

#[no_mangle]
pub extern "C" fn next(range_ptr: *mut c_void) -> bool {
    if range_ptr.is_null() {
        return false;
    }
    let r = range_ptr as *mut Range;
    unsafe {
        (*r).current += 1.0;
        (*r).current < (*r).max
    }
}

#[no_mangle]
pub extern "C" fn current(range_ptr: *mut c_void) -> c_double {
    if range_ptr.is_null() {
        return 0.0;
    }
    let r = range_ptr as *mut Range;
    unsafe { (*r).current }
}

// =============================================================================
// VECTOR OPERATIONS
// =============================================================================

#[no_mangle]
pub extern "C" fn hulk_vector_new(capacity: c_long) -> *mut c_void {
    let vec = unsafe { libc::malloc(std::mem::size_of::<HulkVector>()) as *mut HulkVector };
    unsafe {
        (*vec).size = 0;
        (*vec).capacity = if capacity > 0 { capacity } else { 4 };
        let data_size = ((*vec).capacity as usize) * std::mem::size_of::<*mut c_void>();
        (*vec).data = libc::malloc(data_size) as *mut *mut c_void;
    }
    vec as *mut c_void
}

#[no_mangle]
pub extern "C" fn hulk_vector_push(vec_ptr: *mut c_void, element: *mut c_void) {
    if vec_ptr.is_null() {
        return;
    }
    let vec = vec_ptr as *mut HulkVector;
    unsafe {
        if (*vec).size >= (*vec).capacity {
            let new_cap = (*vec).capacity * 2;
            let new_size = (new_cap as usize) * std::mem::size_of::<*mut c_void>();
            (*vec).data = libc::realloc((*vec).data as *mut c_void, new_size) as *mut *mut c_void;
            (*vec).capacity = new_cap;
        }
        let idx = (*vec).size as isize;
        *((*vec).data.offset(idx)) = element;
        (*vec).size += 1;
    }
}

#[no_mangle]
pub extern "C" fn hulk_vector_get(vec_ptr: *mut c_void, index: c_long) -> *mut c_void {
    if vec_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let vec = vec_ptr as *mut HulkVector;
    unsafe {
        if index < 0 || index >= (*vec).size {
            return std::ptr::null_mut();
        }
        *((*vec).data.offset(index as isize))
    }
}

#[no_mangle]
pub extern "C" fn hulk_vector_size(vec_ptr: *mut c_void) -> c_long {
    if vec_ptr.is_null() {
        return 0;
    }
    let vec = vec_ptr as *mut HulkVector;
    unsafe { (*vec).size }
}

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================

#[no_mangle]
pub extern "C" fn main() -> c_int {
    unsafe { __entry() as c_int }
}
