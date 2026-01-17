// Decompiled to Rust from WebAssembly

fn func_0(idx: i32) -> i32 {
    let mut v: i32 = 0;
    'label$0: {
        {
            if (idx >= 1) { break 'label$0 };
            return 0;
        }
    }
    ({ v = (idx + -1); } + ((((v as u64) * ((idx + -2) as u64)) >> 1i64) as i32))
}
