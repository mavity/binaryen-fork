#![allow(non_camel_case_types)]
use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};

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

// Hash helper to compute ahash of a byte buffer
#[no_mangle]
pub extern "C" fn BinaryenAhashBytes(data: *const c_uchar, len: usize) -> u64 {
    if data.is_null() { return 0; }
    unsafe {
        let slice = std::slice::from_raw_parts(data, len);
        return binaryen_support::hash::ahash_bytes(slice);
    }
}

// FastHashMap FFI helpers (String -> u64)
#[repr(C)]
pub struct BinaryenFastHashMap { _private: [u8; 0] }

#[no_mangle]
pub extern "C" fn BinaryenFastHashMapCreate() -> *mut BinaryenFastHashMap {
    let m: Box<binaryen_support::hash::FastHashMap<String, u64>> = Box::new(Default::default());
    Box::into_raw(m) as *mut BinaryenFastHashMap
}

#[no_mangle]
pub extern "C" fn BinaryenFastHashMapDispose(p: *mut BinaryenFastHashMap) {
    if p.is_null() { return; }
    unsafe { let _ = Box::from_raw(p as *mut binaryen_support::hash::FastHashMap<String, u64>); }
}

#[no_mangle]
pub extern "C" fn BinaryenFastHashMapInsert(
    p: *mut BinaryenFastHashMap,
    key: *const c_char,
    value: u64,
) -> bool {
    if p.is_null() || key.is_null() { return false; }
    unsafe {
        let map = &mut *(p as *mut binaryen_support::hash::FastHashMap<String, u64>);
        if let Ok(s) = CStr::from_ptr(key).to_str() {
            map.insert(s.to_string(), value);
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn BinaryenFastHashMapGet(
    p: *mut BinaryenFastHashMap,
    key: *const c_char,
    out_value: *mut u64,
) -> bool {
    if p.is_null() || key.is_null() || out_value.is_null() { return false; }
    unsafe {
        let map = &*(p as *mut binaryen_support::hash::FastHashMap<String, u64>);
        if let Ok(s) = CStr::from_ptr(key).to_str() {
            if let Some(v) = map.get(s) {
                *out_value = *v;
                return true;
            }
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn BinaryenFastHashMapLen(p: *mut BinaryenFastHashMap) -> usize {
    if p.is_null() { return 0; }
    unsafe {
        let map = &*(p as *mut binaryen_support::hash::FastHashMap<String, u64>);
        map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn test_ffi_version() {
        assert_eq!(binaryen_ffi_version(), 1);
        assert_eq!(binaryen_ffi_abi_version(), BINARYEN_FFI_ABI_VERSION);
    }

    #[test]
    fn test_ffi_echo_and_null() {
        let cs = CString::new("hello").unwrap();
        let out = binaryen_ffi_echo(cs.as_ptr());
        assert_eq!(out, cs.as_ptr());
        assert!(binaryen_ffi_echo(std::ptr::null()) == std::ptr::null());
    }

    #[test]
    fn test_ffi_interner_and_arena() {
        let it = BinaryenStringInternerCreate();
        assert!(!it.is_null());
        let s = CString::new("world").unwrap();
        let p1 = BinaryenStringInternerIntern(it, s.as_ptr());
        let p2 = BinaryenStringInternerIntern(it, s.as_ptr());
        assert_eq!(p1, p2);
        BinaryenStringInternerDispose(it);

        let a = BinaryenArenaCreate();
        assert!(!a.is_null());
        let q = CString::new("arena-test").unwrap();
        let ap = BinaryenArenaAllocString(a, q.as_ptr());
        assert!(!ap.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(ap).to_str().unwrap(), "arena-test");
        }
        BinaryenArenaDispose(a);
    }

    #[test]
    fn test_ffi_ahash() {
        let s = CString::new("hello").unwrap();
        let out = BinaryenAhashBytes(s.as_ptr() as *const u8, 5);
        assert_ne!(out, 0);
        let out2 = BinaryenAhashBytes(s.as_ptr() as *const u8, 5);
        assert_eq!(out, out2);
    }

    #[test]
    fn test_ffi_fast_hashmap() {
        let map = BinaryenFastHashMapCreate();
        assert!(!map.is_null());
        assert!(BinaryenFastHashMapInsert(map, CString::new("k1").unwrap().as_ptr(), 100));
        assert!(BinaryenFastHashMapInsert(map, CString::new("k2").unwrap().as_ptr(), 200));
        let mut outv: u64 = 0;
        assert!(BinaryenFastHashMapGet(map, CString::new("k1").unwrap().as_ptr(), &mut outv as *mut u64));
        assert_eq!(outv, 100);
        let len = BinaryenFastHashMapLen(map);
        assert_eq!(len, 2);
        BinaryenFastHashMapDispose(map);
    }
}
