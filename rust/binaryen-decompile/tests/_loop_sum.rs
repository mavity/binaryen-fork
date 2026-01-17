#[no_mangle]
pub extern "C" fn loop_sum(n: i32) -> i32 {
    let mut sum = 0;
    let mut i = 0;
    while i < n {
        sum += i;
        i += 1;
    }
    sum
}
