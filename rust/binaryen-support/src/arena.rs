use bumpalo::Bump;
use std::ffi::CString;
use std::os::raw::c_char;

pub struct Arena {
    bump: Bump,
}

impl Arena {
    pub fn new() -> Self {
        Arena { bump: Bump::new() }
    }

    /// Allocate a string in the arena and return a pointer valid for the arena lifetime.
    pub fn alloc_str(&self, s: &str) -> *const c_char {
        // allocate a CString containing the data; leak it into the bump
        let bytes = s.as_bytes();
        // Allocate string bytes in bump and return C string pointer. To ensure
        // a null-terminated C string, create a temporary vector with an extra
        // null byte and copy it into bump memory.
        let mut tmp: Vec<u8> = Vec::with_capacity(bytes.len() + 1);
        tmp.extend_from_slice(bytes);
        tmp.push(0u8);
        let vec = self.bump.alloc_slice_copy(&tmp);
        // create a raw pointer to vec's data
        // SAFETY: we ensure a null terminator is present; we won't free this until arena drops
        let ptr = vec.as_mut_ptr() as *mut c_char;
        ptr as *const c_char
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn arena_alloc_string() {
        let a = Arena::new();
        let p1 = a.alloc_str("hello");
        let p2 = a.alloc_str("hello");
        assert_ne!(p1, std::ptr::null());
        assert_ne!(p2, std::ptr::null());
        unsafe {
            use std::ffi::CStr;
            assert_eq!(CStr::from_ptr(p1).to_str().unwrap(), "hello");
            assert_eq!(CStr::from_ptr(p2).to_str().unwrap(), "hello");
        }
    }

    proptest! {
        #[test]
        fn arena_alloc_property_returns_equal_string(s in any::<String>()) {
            proptest::prop_assume!(!s.contains('\0'));
            let a = Arena::new();
            let _cs = std::ffi::CString::new(s.clone()).unwrap();
            let p = a.alloc_str(&s);
            assert!(!p.is_null());
            unsafe { prop_assert_eq!(std::ffi::CStr::from_ptr(p).to_str().unwrap(), s); }
        }
    }
}
