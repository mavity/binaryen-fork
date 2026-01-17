// Decompiled to Rust from WebAssembly

fn func_0(idx0: i32) -> i32 {
    let mut idx1: i32 = 0;
    let mut flag: i32 = 0;
    let mut v: i32 = 0;
    'label$0: {
        {
            if (idx0 >= 2) { break 'label$0 };
            return (idx0 + 0);
        }
    }
    idx1 = 0;
    'label$1: loop     {
        idx1 = (func_0((idx0 + -1)) + idx1);
        if flag { break 'label$1 { idx0 = { v = (idx0 + -2); }; } };
    }
    (v + idx1)
}
