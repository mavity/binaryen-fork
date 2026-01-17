#[no_mangle]
pub extern "C" fn fact(n: i32) -> i32 {
    let mut f = 1;
    for i in 1..=n {
        f *= i;
    }
    f
}
