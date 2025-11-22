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

    // Additional edge case tests for FFI safety
    #[test]
    fn test_ffi_null_safety_interner() {
        // Test null pointer handling
        BinaryenStringInternerDispose(std::ptr::null_mut());
        let it = BinaryenStringInternerCreate();
        assert_eq!(BinaryenStringInternerIntern(std::ptr::null_mut(), std::ptr::null()), std::ptr::null());
        assert_eq!(BinaryenStringInternerIntern(it, std::ptr::null()), std::ptr::null());
        BinaryenStringInternerDispose(it);
    }

    #[test]
    fn test_ffi_null_safety_arena() {
        // Test null pointer handling for arena
        BinaryenArenaDispose(std::ptr::null_mut());
        let a = BinaryenArenaCreate();
        assert_eq!(BinaryenArenaAllocString(std::ptr::null_mut(), std::ptr::null()), std::ptr::null());
        assert_eq!(BinaryenArenaAllocString(a, std::ptr::null()), std::ptr::null());
        BinaryenArenaDispose(a);
    }

    #[test]
    fn test_ffi_null_safety_hashmap() {
        // Test null pointer handling for hashmap
        BinaryenFastHashMapDispose(std::ptr::null_mut());
        let map = BinaryenFastHashMapCreate();
        assert!(!BinaryenFastHashMapInsert(std::ptr::null_mut(), CString::new("k").unwrap().as_ptr(), 1));
        assert!(!BinaryenFastHashMapInsert(map, std::ptr::null(), 1));
        let mut outv: u64 = 0;
        assert!(!BinaryenFastHashMapGet(std::ptr::null_mut(), CString::new("k").unwrap().as_ptr(), &mut outv));
        assert!(!BinaryenFastHashMapGet(map, std::ptr::null(), &mut outv));
        assert!(!BinaryenFastHashMapGet(map, CString::new("k").unwrap().as_ptr(), std::ptr::null_mut()));
        assert_eq!(BinaryenFastHashMapLen(std::ptr::null_mut()), 0);
        BinaryenFastHashMapDispose(map);
    }

    #[test]
    fn test_ffi_hashmap_overwrite() {
        // Test that inserting the same key overwrites the value
        let map = BinaryenFastHashMapCreate();
        assert!(BinaryenFastHashMapInsert(map, CString::new("key").unwrap().as_ptr(), 100));
        let mut outv: u64 = 0;
        assert!(BinaryenFastHashMapGet(map, CString::new("key").unwrap().as_ptr(), &mut outv));
        assert_eq!(outv, 100);
        // Overwrite with new value
        assert!(BinaryenFastHashMapInsert(map, CString::new("key").unwrap().as_ptr(), 200));
        assert!(BinaryenFastHashMapGet(map, CString::new("key").unwrap().as_ptr(), &mut outv));
        assert_eq!(outv, 200);
        BinaryenFastHashMapDispose(map);
    }

    #[test]
    fn test_ffi_hashmap_missing_key() {
        // Test that getting a missing key returns false
        let map = BinaryenFastHashMapCreate();
        let mut outv: u64 = 0;
        assert!(!BinaryenFastHashMapGet(map, CString::new("missing").unwrap().as_ptr(), &mut outv));
        BinaryenFastHashMapDispose(map);
    }

    #[test]
    fn test_ffi_interner_multiple_strings() {
        // Test interning multiple different strings
        let it = BinaryenStringInternerCreate();
        let s1 = CString::new("str1").unwrap();
        let s2 = CString::new("str2").unwrap();
        let s3 = CString::new("str3").unwrap();
        let p1 = BinaryenStringInternerIntern(it, s1.as_ptr());
        let p2 = BinaryenStringInternerIntern(it, s2.as_ptr());
        let p3 = BinaryenStringInternerIntern(it, s3.as_ptr());
        // All should be non-null and different
        assert!(!p1.is_null());
        assert!(!p2.is_null());
        assert!(!p3.is_null());
        assert_ne!(p1, p2);
        assert_ne!(p2, p3);
        assert_ne!(p1, p3);
        // Re-interning same strings should return same pointers
        assert_eq!(p1, BinaryenStringInternerIntern(it, s1.as_ptr()));
        assert_eq!(p2, BinaryenStringInternerIntern(it, s2.as_ptr()));
        assert_eq!(p3, BinaryenStringInternerIntern(it, s3.as_ptr()));
        BinaryenStringInternerDispose(it);
    }

    #[test]
    fn test_ffi_arena_multiple_allocations() {
        // Test allocating multiple strings in arena
        let a = BinaryenArenaCreate();
        let strs = ["one", "two", "three", "four", "five"];
        let mut ptrs = Vec::new();
        for s in &strs {
            let cs = CString::new(*s).unwrap();
            let p = BinaryenArenaAllocString(a, cs.as_ptr());
            assert!(!p.is_null());
            unsafe {
                assert_eq!(CStr::from_ptr(p).to_str().unwrap(), *s);
            }
            ptrs.push(p);
        }
        // All pointers should be different (arena allocates new space each time)
        for i in 0..ptrs.len() {
            for j in (i+1)..ptrs.len() {
                assert_ne!(ptrs[i], ptrs[j]);
            }
        }
        BinaryenArenaDispose(a);
    }

    #[test]
    fn test_ffi_ahash_empty_and_various_sizes() {
        // Test hashing empty data
        let h0 = BinaryenAhashBytes(std::ptr::null(), 0);
        assert_eq!(h0, 0); // null pointer returns 0
        
        // Test various sizes
        let data = b"test data for hashing with various sizes";
        let h1 = BinaryenAhashBytes(data.as_ptr(), 1);
        let h2 = BinaryenAhashBytes(data.as_ptr(), 5);
        let h3 = BinaryenAhashBytes(data.as_ptr(), data.len());
        
        // Different lengths should generally produce different hashes
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        
        // Same data and length should produce same hash (deterministic)
        assert_eq!(h1, BinaryenAhashBytes(data.as_ptr(), 1));
        assert_eq!(h2, BinaryenAhashBytes(data.as_ptr(), 5));
        assert_eq!(h3, BinaryenAhashBytes(data.as_ptr(), data.len()));
    }

    #[test]
    fn test_ffi_abi_version_consistency() {
        // Ensure runtime ABI version matches compile-time constant
        assert_eq!(binaryen_ffi_abi_version(), BINARYEN_FFI_ABI_VERSION);
        assert_eq!(BINARYEN_FFI_ABI_VERSION, 1); // Current expected version
    }
}
