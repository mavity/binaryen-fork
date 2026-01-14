use crate::Type;
use std::fmt;

#[derive(Clone, PartialEq)]
pub enum Literal {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    // TODO: V128 and Reference types
}

impl Literal {
    pub fn get_type(&self) -> Type {
        match self {
            Literal::I32(_) => Type::I32,
            Literal::I64(_) => Type::I64,
            Literal::F32(_) => Type::F32,
            Literal::F64(_) => Type::F64,
        }
    }
}

impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::I32(v) => write!(f, "i32.const {}", v),
            Literal::I64(v) => write!(f, "i64.const {}", v),
            Literal::F32(v) => write!(f, "f32.const {}", v),
            Literal::F64(v) => write!(f, "f64.const {}", v),
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         match self {
            Literal::I32(v) => write!(f, "{}", v),
            Literal::I64(v) => write!(f, "{}", v),
            Literal::F32(v) => write!(f, "{}", v),
            Literal::F64(v) => write!(f, "{}", v),
        }
    }
}
