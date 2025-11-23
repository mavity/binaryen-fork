#![no_main]
use libfuzzer_sys::fuzz_target;
use std::str;
use binaryen_support::StringInterner;

fuzz_target!(|data: &[u8]| {
    // We try to convert to UTF-8; if it fails, still test that the interner
    // does not panic on invalid input — we only intern valid strings.
    if let Ok(s) = str::from_utf8(data) {
        // Create interner, intern the string; also briefly exercise concurrent
        // interning for small bursts to exercise race paths.
        let interner = StringInterner::new();
        let _ = interner.intern(s);
        // concurrent interning burst
        let arc = std::sync::Arc::new(interner);
        let mut threads = Vec::new();
        for _ in 0..4 {
            let a = arc.clone();
            let local = s.to_string();
            threads.push(std::thread::spawn(move || {
                let _ = a.intern(&local);
            }));
        }
        for t in threads { let _ = t.join(); }
    }
});
