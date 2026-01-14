use crate::ops::{BinaryOp, UnaryOp};
use binaryen_core::{Literal, Type};
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use std::fmt;

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
    Nop,
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
}
