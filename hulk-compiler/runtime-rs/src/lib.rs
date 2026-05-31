use std::os::raw::{c_char, c_long, c_void};

#[no_mangle]
pub extern "C" fn print(ptr: *const c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        // Use libc printf to avoid pulling extra Rust-formatting dependencies at link time
        libc::printf(b"%s\0".as_ptr() as *const c_char, ptr);
    }
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
