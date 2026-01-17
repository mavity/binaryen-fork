#[no_mangle]
pub extern "C" fn max(a: i32, b: i32) -> i32 {
    if a > b {
        a
    } else {
        b
    }
}
