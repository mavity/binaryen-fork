#![no_main]
use libfuzzer_sys::fuzz_target;
use binaryen_support::hash::FastHashMap;
use arbitrary::Arbitrary;
use std::collections::VecDeque;

#[derive(Arbitrary, Debug)]
enum Op {
    Insert(String, u64),
    Get(String),
    Remove(String),
}

fuzz_target!(|data: &[u8]| {
    // Interpret bytes as a sequence of operations
    if let Ok(seq) = arbitrary::Unstructured::new(data).arbitrary::<VecDeque<Op>>() {
        let mut map: FastHashMap<String, u64> = FastHashMap::default();
        for op in seq.into_iter().take(512) {
            match op {
                Op::Insert(k, v) => {
                    map.insert(k, v);
                }
                Op::Get(k) => {
                    let _ = map.get(&k);
                }
                Op::Remove(k) => {
                    let _ = map.remove(&k);
                }
            }
        }
    }
});
