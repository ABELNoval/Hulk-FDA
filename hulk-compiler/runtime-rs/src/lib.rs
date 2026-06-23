use std::os::raw::{c_char, c_double, c_int, c_long, c_void};

extern "C" {
    fn __entry() -> c_long;
}

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
        // +1 por el espacio, +1 por el caracter nulo
        let ptr = libc::malloc(len1 + len2 + 2) as *mut c_char;
        libc::sprintf(ptr, b"%s %s\0".as_ptr() as *const c_char, s1, s2);
        ptr
    }
}

#[no_mangle]
pub extern "C" fn hulk_num_to_str(num: c_double) -> *const c_char {
    unsafe {
        // Reservar suficiente espacio para un número flotante (32 bytes es seguro)
        let ptr = libc::malloc(32) as *mut c_char;
        libc::sprintf(ptr, b"%g\0".as_ptr() as *const c_char, num);
        ptr
    }
}

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
pub extern "C" fn main() -> c_int {
    unsafe { __entry() as c_int }
}

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
