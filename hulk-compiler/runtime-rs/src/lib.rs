use std::os::raw::{c_char, c_double, c_int, c_long, c_void};

extern "C" {
    fn __entry() -> c_long;
}

#[no_mangle]
pub extern "C" fn print(value: c_double) {
    unsafe {
        // Use libc printf to avoid pulling extra Rust-formatting dependencies at link time
        libc::printf(b"%g\n\0".as_ptr() as *const c_char, value);
    }
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
