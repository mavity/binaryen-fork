use crate::ops::{BinaryOp, UnaryOp};
use binaryen_core::{Literal, Type};
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;

pub type ExprRef<'a> = &'a mut Expression<'a>;

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
}

impl<'a> Expression<'a> {
    pub fn new(bump: &'a Bump, kind: ExpressionKind<'a>, type_: Type) -> ExprRef<'a> {
        bump.alloc(Expression { kind, type_ })
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
}
