// Minimal support crate skeleton

pub fn version() -> &'static str {
    "binaryen-support-0.1"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(version(), "binaryen-support-0.1");
    }
}
