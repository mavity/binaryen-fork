#![allow(non_camel_case_types)]
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn binaryen_ffi_version() -> u32 {
    // Return a simple version identifier for smoke tests
    1
}

// ABI version macro; bump this when changing any exported symbols or layouts.
// This is intentionally `pub` so `cbindgen` can emit a corresponding macro in
// the generated `include/binaryen_ffi.h` header.
pub const BINARYEN_FFI_ABI_VERSION: u32 = 1;

#[no_mangle]
pub extern "C" fn binaryen_ffi_abi_version() -> u32 {
    BINARYEN_FFI_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn binaryen_ffi_echo(s: *const c_char) -> *const c_char {
    if s.is_null() {
        return std::ptr::null();
    }
    unsafe {
        let cstr = CStr::from_ptr(s);
        if let Ok(_str_slice) = cstr.to_str() {
            // Return the same pointer as a noop (not ideal lifetime, but ok for smoke test)
            return s;
        }
    }
    std::ptr::null()
}

// String interner FFI helpers
#[repr(C)]
pub struct BinaryenStringInterner { _private: [u8; 0] }

#[no_mangle]
pub extern "C" fn BinaryenStringInternerCreate() -> *mut BinaryenStringInterner {
    let interner = Box::new(binaryen_support::StringInterner::new());
    Box::into_raw(interner) as *mut BinaryenStringInterner
}

#[no_mangle]
pub extern "C" fn BinaryenStringInternerDispose(p: *mut BinaryenStringInterner) {
    if p.is_null() { return; }
    unsafe { let _ = Box::from_raw(p as *mut binaryen_support::StringInterner); }
}

#[no_mangle]
pub extern "C" fn BinaryenStringInternerIntern(
    p: *mut BinaryenStringInterner,
    s: *const c_char,
) -> *const c_char {
    if p.is_null() || s.is_null() { return std::ptr::null(); }
    unsafe {
        let interner = &*(p as *mut binaryen_support::StringInterner);
        if let Ok(str_slice) = CStr::from_ptr(s).to_str() {
            let interned = interner.intern(str_slice);
            return interned.as_ptr() as *const c_char;
        }
    }
    std::ptr::null()
}

// Arena FFI helpers
#[repr(C)]
pub struct BinaryenArena { _private: [u8; 0] }

#[no_mangle]
pub extern "C" fn BinaryenArenaCreate() -> *mut BinaryenArena {
    let arena = Box::new(binaryen_support::Arena::new());
    Box::into_raw(arena) as *mut BinaryenArena
}

#[no_mangle]
pub extern "C" fn BinaryenArenaDispose(p: *mut BinaryenArena) {
    if p.is_null() { return; }
    unsafe { let _ = Box::from_raw(p as *mut binaryen_support::Arena); }
}

#[no_mangle]
pub extern "C" fn BinaryenArenaAllocString(p: *mut BinaryenArena, s: *const c_char) -> *const c_char {
    if p.is_null() || s.is_null() { return std::ptr::null(); }
    unsafe {
        let arena = &*(p as *mut binaryen_support::Arena);
        if let Ok(str_slice) = CStr::from_ptr(s).to_str() {
            return arena.alloc_str(str_slice);
        }
    }
    std::ptr::null()
}
