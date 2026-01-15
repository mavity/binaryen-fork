use crate::ops::{BinaryOp, UnaryOp};
use binaryen_core::{Literal, Type};
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ExprRef<'a>(NonNull<Expression<'a>>);

impl<'a> ExprRef<'a> {
    pub fn new(ptr: &'a mut Expression<'a>) -> Self {
        Self(NonNull::from(ptr))
    }

    pub fn as_ptr(&self) -> *mut Expression<'a> {
        self.0.as_ptr()
    }
}

unsafe impl<'a> Send for ExprRef<'a> {}
unsafe impl<'a> Sync for ExprRef<'a> {}

impl<'a> Deref for ExprRef<'a> {
    type Target = Expression<'a>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<'a> DerefMut for ExprRef<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

#[derive(Debug)]
pub struct Expression<'a> {
    pub type_: Type,
    pub kind: ExpressionKind<'a>,
}

#[derive(Debug)]
pub enum ExpressionKind<'a> {
    Block {
        name: Option<&'a str>,
        list: BumpVec<'a, ExprRef<'a>>,
    },
    Const(Literal),
    Unary {
        op: UnaryOp,
        value: ExprRef<'a>,
    },
    Binary {
        op: BinaryOp,
        left: ExprRef<'a>,
        right: ExprRef<'a>,
    },
    Call {
        target: &'a str,
        operands: BumpVec<'a, ExprRef<'a>>,
        is_return: bool,
    },
    LocalGet {
        index: u32,
    },
    LocalSet {
        index: u32,
        value: ExprRef<'a>,
    },
    LocalTee {
        index: u32,
        value: ExprRef<'a>,
    },
    GlobalGet {
        index: u32,
    },
    GlobalSet {
        index: u32,
        value: ExprRef<'a>,
    },
    If {
        condition: ExprRef<'a>,
        if_true: ExprRef<'a>,
        if_false: Option<ExprRef<'a>>,
    },
    Loop {
        name: Option<&'a str>,
        body: ExprRef<'a>,
    },
    Break {
        name: &'a str,
        condition: Option<ExprRef<'a>>,
        value: Option<ExprRef<'a>>,
    },
    Return {
        value: Option<ExprRef<'a>>,
    },
    Unreachable,
    Drop {
        value: ExprRef<'a>,
    },
    Select {
        condition: ExprRef<'a>,
        if_true: ExprRef<'a>,
        if_false: ExprRef<'a>,
    },
    Load {
        bytes: u32,       // 1, 2, 4, or 8
        signed: bool,     // For sub-word loads
        offset: u32,      // Memory offset
        align: u32,       // Alignment (power of 2)
        ptr: ExprRef<'a>, // Address to load from
    },
    Store {
        bytes: u32,         // 1, 2, 4, or 8
        offset: u32,        // Memory offset
        align: u32,         // Alignment
        ptr: ExprRef<'a>,   // Address to store to
        value: ExprRef<'a>, // Value to store
    },
    Nop,
    Switch {
        names: BumpVec<'a, &'a str>, // List of target labels
        default: &'a str,            // Default label
        condition: ExprRef<'a>,      // Index
        value: Option<ExprRef<'a>>,  // Value passing (nullable)
    },
    CallIndirect {
        table: &'a str,
        target: ExprRef<'a>, // Function index
        operands: BumpVec<'a, ExprRef<'a>>,
        type_: Type, // Signature
    },
    MemoryGrow {
        delta: ExprRef<'a>,
    },
    MemorySize,
    // Atomic operations
    AtomicRMW {
        op: crate::ops::AtomicOp,
        bytes: u32,
        offset: u32,
        ptr: ExprRef<'a>,
        value: ExprRef<'a>,
    },
    AtomicCmpxchg {
        bytes: u32,
        offset: u32,
        ptr: ExprRef<'a>,
        expected: ExprRef<'a>,
        replacement: ExprRef<'a>,
    },
    AtomicWait {
        ptr: ExprRef<'a>,
        expected: ExprRef<'a>,
        timeout: ExprRef<'a>,
        expected_type: Type,
    },
    AtomicNotify {
        ptr: ExprRef<'a>,
        count: ExprRef<'a>,
    },
    AtomicFence,
    // SIMD operations
    SIMDExtract {
        op: crate::ops::SIMDOp,
        vec: ExprRef<'a>,
        index: u8,
    },
    SIMDReplace {
        op: crate::ops::SIMDOp,
        vec: ExprRef<'a>,
        index: u8,
        value: ExprRef<'a>,
    },
    SIMDShuffle {
        left: ExprRef<'a>,
        right: ExprRef<'a>,
        mask: [u8; 16],
    },
    SIMDTernary {
        op: crate::ops::SIMDOp,
        a: ExprRef<'a>,
        b: ExprRef<'a>,
        c: ExprRef<'a>,
    },
    SIMDShift {
        op: crate::ops::SIMDOp,
        vec: ExprRef<'a>,
        shift: ExprRef<'a>,
    },
    SIMDLoad {
        op: crate::ops::SIMDOp,
        offset: u32,
        align: u32,
        ptr: ExprRef<'a>,
    },
    SIMDLoadStoreLane {
        is_store: bool,
        op: crate::ops::SIMDOp,
        offset: u32,
        align: u32,
        index: u8,
        ptr: ExprRef<'a>,
        vec: ExprRef<'a>,
    },
    // Bulk memory operations
    MemoryInit {
        segment: u32,
        dest: ExprRef<'a>,
        offset: ExprRef<'a>,
        size: ExprRef<'a>,
    },
    DataDrop {
        segment: u32,
    },
    MemoryCopy {
        dest: ExprRef<'a>,
        src: ExprRef<'a>,
        size: ExprRef<'a>,
    },
    MemoryFill {
        dest: ExprRef<'a>,
        value: ExprRef<'a>,
        size: ExprRef<'a>,
    },
}

impl<'a> Expression<'a> {
    pub fn new(bump: &'a Bump, kind: ExpressionKind<'a>, type_: Type) -> ExprRef<'a> {
        ExprRef::new(bump.alloc(Expression { kind, type_ }))
    }
}

// Helpers for construction
pub struct IrBuilder<'a> {
    pub bump: &'a Bump,
}

impl<'a> IrBuilder<'a> {
    pub fn new(bump: &'a Bump) -> Self {
        Self { bump }
    }

    pub fn nop(&self) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Nop, Type::NONE)
    }

    pub fn const_(&self, value: Literal) -> ExprRef<'a> {
        let type_ = value.get_type();
        Expression::new(self.bump, ExpressionKind::Const(value), type_)
    }

    pub fn block(
        &self,
        name: Option<&'a str>,
        list: BumpVec<'a, ExprRef<'a>>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Block { name, list }, type_)
    }

    pub fn unary(&self, op: UnaryOp, value: ExprRef<'a>, type_: Type) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Unary { op, value }, type_)
    }

    pub fn binary(
        &self,
        op: BinaryOp,
        left: ExprRef<'a>,
        right: ExprRef<'a>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Binary { op, left, right }, type_)
    }

    pub fn drop(&self, value: ExprRef<'a>) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Drop { value }, Type::NONE)
    }

    pub fn call(
        &self,
        target: &'a str,
        operands: BumpVec<'a, ExprRef<'a>>,
        type_: Type,
        is_return: bool,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::Call {
                target,
                operands,
                is_return,
            },
            type_,
        )
    }

    pub fn local_get(&self, index: u32, type_: Type) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::LocalGet { index }, type_)
    }

    pub fn local_set(&self, index: u32, value: ExprRef<'a>) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::LocalSet { index, value },
            Type::NONE,
        )
    }

    pub fn local_tee(&self, index: u32, value: ExprRef<'a>, type_: Type) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::LocalTee { index, value }, type_)
    }

    pub fn global_get(&self, index: u32, type_: Type) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::GlobalGet { index }, type_)
    }

    pub fn global_set(&self, index: u32, value: ExprRef<'a>) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::GlobalSet { index, value },
            Type::NONE,
        )
    }

    pub fn if_(
        &self,
        condition: ExprRef<'a>,
        if_true: ExprRef<'a>,
        if_false: Option<ExprRef<'a>>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            },
            type_,
        )
    }

    pub fn loop_(&self, name: Option<&'a str>, body: ExprRef<'a>, type_: Type) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Loop { name, body }, type_)
    }

    pub fn break_(
        &self,
        name: &'a str,
        condition: Option<ExprRef<'a>>,
        value: Option<ExprRef<'a>>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::Break {
                name,
                condition,
                value,
            },
            type_,
        )
    }

    pub fn load(
        &self,
        bytes: u32,
        signed: bool,
        offset: u32,
        align: u32,
        ptr: ExprRef<'a>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::Load {
                bytes,
                signed,
                offset,
                align,
                ptr,
            },
            type_,
        )
    }

    pub fn store(
        &self,
        bytes: u32,
        offset: u32,
        align: u32,
        ptr: ExprRef<'a>,
        value: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::Store {
                bytes,
                offset,
                align,
                ptr,
                value,
            },
            Type::NONE,
        )
    }

    pub fn return_(&self, value: Option<ExprRef<'a>>) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Return { value }, Type::NONE)
    }

    pub fn unreachable(&self) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::Unreachable, Type::NONE)
    }

    pub fn select(
        &self,
        condition: ExprRef<'a>,
        if_true: ExprRef<'a>,
        if_false: ExprRef<'a>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            },
            type_,
        )
    }

    pub fn memory_size(&self) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::MemorySize, Type::I32)
    }

    pub fn memory_grow(&self, delta: ExprRef<'a>) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::MemoryGrow { delta }, Type::I32)
    }

    pub fn atomic_rmw(
        &self,
        op: crate::ops::AtomicOp,
        bytes: u32,
        offset: u32,
        ptr: ExprRef<'a>,
        value: ExprRef<'a>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::AtomicRMW {
                op,
                bytes,
                offset,
                ptr,
                value,
            },
            type_,
        )
    }

    pub fn atomic_cmpxchg(
        &self,
        bytes: u32,
        offset: u32,
        ptr: ExprRef<'a>,
        expected: ExprRef<'a>,
        replacement: ExprRef<'a>,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::AtomicCmpxchg {
                bytes,
                offset,
                ptr,
                expected,
                replacement,
            },
            type_,
        )
    }

    pub fn atomic_wait(
        &self,
        ptr: ExprRef<'a>,
        expected: ExprRef<'a>,
        timeout: ExprRef<'a>,
        expected_type: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::AtomicWait {
                ptr,
                expected,
                timeout,
                expected_type,
            },
            Type::I32,
        )
    }

    pub fn atomic_notify(&self, ptr: ExprRef<'a>, count: ExprRef<'a>) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::AtomicNotify { ptr, count },
            Type::I32,
        )
    }

    pub fn atomic_fence(&self) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::AtomicFence, Type::NONE)
    }

    pub fn memory_init(
        &self,
        segment: u32,
        dest: ExprRef<'a>,
        offset: ExprRef<'a>,
        size: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::MemoryInit {
                segment,
                dest,
                offset,
                size,
            },
            Type::NONE,
        )
    }

    pub fn data_drop(&self, segment: u32) -> ExprRef<'a> {
        Expression::new(self.bump, ExpressionKind::DataDrop { segment }, Type::NONE)
    }

    pub fn memory_copy(
        &self,
        dest: ExprRef<'a>,
        src: ExprRef<'a>,
        size: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::MemoryCopy { dest, src, size },
            Type::NONE,
        )
    }

    pub fn memory_fill(
        &self,
        dest: ExprRef<'a>,
        value: ExprRef<'a>,
        size: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::MemoryFill { dest, value, size },
            Type::NONE,
        )
    }

    pub fn simd_extract(
        &self,
        op: crate::ops::SIMDOp,
        vec: ExprRef<'a>,
        index: u8,
        type_: Type,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::SIMDExtract { op, vec, index },
            type_,
        )
    }

    pub fn simd_replace(
        &self,
        op: crate::ops::SIMDOp,
        vec: ExprRef<'a>,
        index: u8,
        value: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::SIMDReplace {
                op,
                vec,
                index,
                value,
            },
            Type::V128,
        )
    }

    pub fn simd_shuffle(
        &self,
        left: ExprRef<'a>,
        right: ExprRef<'a>,
        mask: [u8; 16],
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::SIMDShuffle { left, right, mask },
            Type::V128,
        )
    }

    pub fn simd_ternary(
        &self,
        op: crate::ops::SIMDOp,
        a: ExprRef<'a>,
        b: ExprRef<'a>,
        c: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::SIMDTernary { op, a, b, c },
            Type::V128,
        )
    }

    pub fn simd_shift(
        &self,
        op: crate::ops::SIMDOp,
        vec: ExprRef<'a>,
        shift: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::SIMDShift { op, vec, shift },
            Type::V128,
        )
    }

    pub fn simd_load(
        &self,
        op: crate::ops::SIMDOp,
        offset: u32,
        align: u32,
        ptr: ExprRef<'a>,
    ) -> ExprRef<'a> {
        Expression::new(
            self.bump,
            ExpressionKind::SIMDLoad {
                op,
                offset,
                align,
                ptr,
            },
            Type::V128,
        )
    }
}
