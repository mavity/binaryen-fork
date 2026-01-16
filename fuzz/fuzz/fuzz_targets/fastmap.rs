#![no_main]
use libfuzzer_sys::fuzz_target;
use binaryen_support::hash::FastHashMap;
use std::string::String;

fuzz_target!(|data: &[u8]| {
    // Interpret bytes as a sequence of simple operations.
    // We use `arbitrary` to extract small strings and values.
    let mut i = 0usize;
    let mut map: FastHashMap<String, u64> = FastHashMap::default();
    while i < data.len() {
        let op_tag = data[i];
        i += 1;
        let op = op_tag as usize % 3;
        match op {
                0 => {
                    if i >= data.len() { break; }
                    let l = data[i] as usize % 32;
                    i += 1;
                    if i + l > data.len() { break; }
                    let s = String::from_utf8_lossy(&data[i..i + l]).to_string();
                    i += l;
                    // read up to 8 bytes for u64
                    let mut val_bytes: [u8; 8] = [0; 8];
                    for j in 0..8 {
                        if i + j < data.len() { val_bytes[j] = data[i + j]; } else { break; }
                    }
                    let v = u64::from_le_bytes(val_bytes);
                    i += 8.min(data.len() - i);
                    map.insert(s, v);
                }
                1 => {
                    if i >= data.len() { break; }
                    let l = data[i] as usize % 32;
                    i += 1;
                    if i + l > data.len() { break; }
                    let s = String::from_utf8_lossy(&data[i..i + l]).to_string();
                    i += l;
                    let _ = map.get(&s);
                }
                2 => {
                    if i >= data.len() { break; }
                    let l = data[i] as usize % 32;
                    i += 1;
                    if i + l > data.len() { break; }
                    let s = String::from_utf8_lossy(&data[i..i + l]).to_string();
                    i += l;
                    let _ = map.remove(&s);
                }
                _ => unreachable!(),
            }
            // Guard against unbounded loop or huge maps
            if map.len() > 4096 {
                map.clear();
            }
        }
});
