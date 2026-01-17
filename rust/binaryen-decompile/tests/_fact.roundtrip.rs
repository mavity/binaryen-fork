// Decompiled to Rust from WebAssembly

fn func_0(len: i32) -> i32 {
    let mut v: i32 = 0;
    let mut idx: i32 = 0;
    v = 1;
    'label$0: {
        {
            if (len < 1) { break 'label$0 };
            v = 1;
            idx = 1;
            'label$1: loop             if ({ idx = (idx + (idx < len)); } <= len) { break 'label$1 if (idx >= len) { break 'label$0 { v = (idx * v); } } };
        }
    }
    v
}
