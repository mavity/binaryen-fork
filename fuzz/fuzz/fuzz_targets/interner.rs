#![no_main]
use libfuzzer_sys::fuzz_target;
use std::str;
use binaryen_support::StringInterner;

fuzz_target!(|data: &[u8]| {
    // We try to convert to UTF-8; if it fails, still test that the interner
    // does not panic on invalid input — we only intern valid strings.
    if let Ok(s) = str::from_utf8(data) {
        let interner = StringInterner::new();
        let _ = interner.intern(s);
    }
});
