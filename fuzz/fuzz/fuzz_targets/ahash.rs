#![no_main]
use libfuzzer_sys::fuzz_target;
use binaryen_support::hash::ahash_bytes;

fuzz_target!(|data: &[u8]| {
    // Feed random bytes into the ahash helper — should never crash.
    let _ = ahash_bytes(data);
});
