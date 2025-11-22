#![no_main]
use libfuzzer_sys::fuzz_target;
use std::str;
use binaryen_support::Arena;

fuzz_target!(|data: &[u8]| {
    // allocate strings of small lengths; the arena should not panic.
    if let Ok(s) = str::from_utf8(data) {
        let a = Arena::new();
        for chunk in s.as_bytes().chunks(64) {
            let _ = a.alloc_str(std::str::from_utf8(chunk).unwrap_or(""));
        }
    }
});
