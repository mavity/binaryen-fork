use std::fmt;

/// Representation of a WebAssembly type.
///
/// This matches the `wasm::Type` representation in C++ Binaryen.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Type(u64);

impl Type {
    // Basic types
    pub const NONE: Type = Type(0);
    pub const UNREACHABLE: Type = Type(1);
    pub const I32: Type = Type(2);
    pub const I64: Type = Type(3);
    pub const F32: Type = Type(4);
    pub const F64: Type = Type(5);
    pub const V128: Type = Type(6);

    // Masks
    const TUPLE_MASK: u64 = 1 << 0;
    const NULL_MASK: u64 = 1 << 1;
    const EXACT_MASK: u64 = 1 << 2;

    // Common Reference Types (Aliases)
    // funcref = nullable func
    pub const FUNCREF: Type = Type((2 << 3) | Self::NULL_MASK);
    // externref = nullable ext
    pub const EXTERNREF: Type = Type((1 << 3) | Self::NULL_MASK);
    // anyref = nullable any
    pub const ANYREF: Type = Type((4 << 3) | Self::NULL_MASK);
    // eqref = nullable eq
    pub const EQREF: Type = Type((5 << 3) | Self::NULL_MASK);
    // i31ref = nullable i31
    pub const I31REF: Type = Type((6 << 3) | Self::NULL_MASK);
    // structref = nullable struct
    pub const STRUCTREF: Type = Type((7 << 3) | Self::NULL_MASK);
    // arrayref = nullable array
    pub const ARRAYREF: Type = Type((8 << 3) | Self::NULL_MASK);
    
    pub fn new(heap_type: HeapType, nullable: bool) -> Self {
        let null_bit = if nullable { Self::NULL_MASK } else { 0 };
        // Note: Exactness logic omitted for basic constructor for now
        Type(heap_type.0 | null_bit)
    }

    pub fn is_basic(self) -> bool {
        self.0 <= 6
    }
    
    pub fn is_nullable(self) -> bool {
        !self.is_basic() && (self.0 & Self::NULL_MASK != 0)
    }

    pub fn is_ref(self) -> bool {
        !self.is_basic() && (self.0 & Self::TUPLE_MASK == 0)
    }
    
    pub fn get_heap_type(self) -> Option<HeapType> {
        if self.is_ref() {
            // Simplified extraction for basic types
            Some(HeapType(self.0 & !Self::NULL_MASK & !Self::EXACT_MASK))
        } else {
            None
        }
    }
}

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Type::NONE => write!(f, "none"),
            Type::UNREACHABLE => write!(f, "unreachable"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::V128 => write!(f, "v128"),
            Type::FUNCREF => write!(f, "funcref"),
            Type::EXTERNREF => write!(f, "externref"),
            Type::ANYREF => write!(f, "anyref"),
            Type::EQREF => write!(f, "eqref"),
            Type::I31REF => write!(f, "i31ref"),
            Type::STRUCTREF => write!(f, "structref"),
            Type::ARRAYREF => write!(f, "arrayref"),
            _ => write!(f, "Type({:#x})", self.0),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HeapType(u64);

impl HeapType {
    const USED_BITS: u64 = 3;
    
    pub const EXT: HeapType = HeapType(1 << 3);
    pub const FUNC: HeapType = HeapType(2 << 3);
    pub const CONT: HeapType = HeapType(3 << 3);
    pub const ANY: HeapType = HeapType(4 << 3);
    pub const EQ: HeapType = HeapType(5 << 3);
    pub const I31: HeapType = HeapType(6 << 3);
    pub const STRUCT: HeapType = HeapType(7 << 3);
    pub const ARRAY: HeapType = HeapType(8 << 3);
    pub const EXN: HeapType = HeapType(9 << 3);
    pub const STRING: HeapType = HeapType(10 << 3);
    pub const NONE: HeapType = HeapType(11 << 3);
    pub const NOEXT: HeapType = HeapType(12 << 3);
    pub const NOFUNC: HeapType = HeapType(13 << 3);
    pub const NOCONT: HeapType = HeapType(14 << 3);
    pub const NOEXN: HeapType = HeapType(15 << 3);

    pub fn is_basic(self) -> bool {
        // This is a simplification, assuming we only have basic types for now
        true 
    }
}

impl fmt::Debug for HeapType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            HeapType::EXT => write!(f, "ext"),
            HeapType::FUNC => write!(f, "func"),
            HeapType::ANY => write!(f, "any"),
            HeapType::EQ => write!(f, "eq"),
            HeapType::I31 => write!(f, "i31"),
            HeapType::STRUCT => write!(f, "struct"),
            HeapType::ARRAY => write!(f, "array"),
            _ => write!(f, "HeapType({:#x})", self.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Signature {
    pub params: Type,
    pub results: Type,
}

impl Signature {
    pub fn new(params: Type, results: Type) -> Self {
        Self { params, results }
    }
}
