#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Type {
    None = 0,
    I32 = 1,
    I64 = 2,
    F32 = 3,
    F64 = 4,
}

impl Type {
    pub fn name(self) -> &'static str {
        match self {
            Type::None => "none",
            Type::I32 => "i32",
            Type::I64 => "i64",
            Type::F32 => "f32",
            Type::F64 => "f64",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_name() {
        assert_eq!(Type::I32.name(), "i32");
        assert_eq!(Type::F64.name(), "f64");
    }

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::I32, Type::I32);
        assert_ne!(Type::I32, Type::I64);
    }
}
