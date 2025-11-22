#![no_main]
use libfuzzer_sys::fuzz_target;
use std::str;
use binaryen_support::Arena;

fuzz_target!(|data: &[u8]| {
    // allocate strings of small lengths; the arena should not panic.
    if let Ok(s) = str::from_utf8(data) {
        let a = Arena::new();
        // Do a burst of allocations; also spawn threads to stress concurrent allocation
        for chunk in s.as_bytes().chunks(64) {
            let _ = a.alloc_str(std::str::from_utf8(chunk).unwrap_or(""));
        }
        // spawn a few threads to exercise concurrent allocations
        let a = std::sync::Arc::new(a);
        let mut handles = Vec::new();
        for _ in 0..4 {
            let aa = a.clone();
            let h = std::thread::spawn(move || {
                for _ in 0..16 {
                    let _ = aa.alloc_str("threaded-fuzz");
                }
            });
            handles.push(h);
        }
        for h in handles { let _ = h.join(); }
    }
});
