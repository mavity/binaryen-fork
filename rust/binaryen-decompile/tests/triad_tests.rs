mod common;

#[cfg(test)]
mod triads {
    use binaryen_macros::triad_tests;

    #[triad_tests]
    mod generated {}
}
