#![allow(non_camel_case_types)]
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn binaryen_ffi_version() -> u32 {
    // Return a simple version identifier for smoke tests
    1
}

#[no_mangle]
pub extern "C" fn binaryen_ffi_echo(s: *const c_char) -> *const c_char {
    if s.is_null() {
        return std::ptr::null();
    }
    unsafe {
        let cstr = CStr::from_ptr(s);
        if let Ok(str_slice) = cstr.to_str() {
            // Return the same pointer as a noop (not ideal lifetime, but ok for smoke test)
            return s;
        }
    }
    std::ptr::null()
}
