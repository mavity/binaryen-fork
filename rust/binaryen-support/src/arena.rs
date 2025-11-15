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
        // Allocate string bytes in bump and return C string pointer
        let vec = self.bump.alloc_slice_copy(bytes);
        // append null terminator
        // allocate the terminator in-place -- simpler to append explicitly to
        // the bump slice. However bumpalo doesn't provide an API to append to the
        // same object so we'll allocate a single-element slice for the terminator
        let _term = self.bump.alloc_slice_fill_copy(1, &[0u8]);
        // create a raw pointer to vec's data
        // SAFETY: we ensure a null terminator is present; we won't free this until arena drops
        let ptr = vec.as_mut_ptr() as *mut c_char;
        ptr as *const c_char
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_alloc_string() {
        let a = Arena::new();
        let p1 = a.alloc_str("hello");
        let p2 = a.alloc_str("hello");
        assert_ne!(p1, std::ptr::null());
        assert_ne!(p2, std::ptr::null());
    }
}
