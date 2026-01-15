use crate::expression::{ExprRef, ExpressionKind};
use crate::ops::{BinaryOp, UnaryOp};
use binaryen_core::Literal;
use bumpalo::Bump;
use std::collections::HashMap;

/// Compile-time expression evaluator
pub struct Evaluator<'a> {
    _arena: &'a Bump,
    /// Known constant values for globals/locals
    /// Using usize for now as GlobalId/LocalId are likely indices
    constants: HashMap<usize, Literal>,
}

impl<'a> Evaluator<'a> {
    pub fn new(arena: &'a Bump) -> Self {
        Self {
            _arena: arena,
            constants: HashMap::new(),
        }
    }

    /// Evaluate expression to constant if possible
    pub fn eval(&self, expr: ExprRef) -> Option<Literal> {
        match &expr.kind {
            ExpressionKind::Const(lit) => Some(lit.clone()),
            ExpressionKind::Binary { op, left, right } => {
                let l = self.eval(*left)?;
                let r = self.eval(*right)?;
                self.eval_binary(*op, l, r)
            }
            ExpressionKind::Unary { op, value } => {
                let v = self.eval(*value)?;
                self.eval_unary(*op, v)
            }
            // TODO: Handle GlobalGet/LocalGet if we have context
            _ => None,
        }
    }

    fn eval_binary(&self, op: BinaryOp, l: Literal, r: Literal) -> Option<Literal> {
        match (op, l, r) {
            // Int32
            (BinaryOp::AddInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(l.wrapping_add(r)))
            }
            (BinaryOp::SubInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(l.wrapping_sub(r)))
            }
            (BinaryOp::MulInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(l.wrapping_mul(r)))
            }
            (BinaryOp::DivSInt32, Literal::I32(l), Literal::I32(r)) => {
                if r == 0 {
                    return None;
                } // Division by zero
                if l == i32::MIN && r == -1 {
                    return None;
                } // Overflow
                Some(Literal::I32(l.wrapping_div(r)))
            }
            (BinaryOp::DivUInt32, Literal::I32(l), Literal::I32(r)) => {
                if r == 0 {
                    return None;
                }
                Some(Literal::I32((l as u32).wrapping_div(r as u32) as i32))
            }
            (BinaryOp::RemSInt32, Literal::I32(l), Literal::I32(r)) => {
                if r == 0 {
                    return None;
                }
                if l == i32::MIN && r == -1 {
                    return Some(Literal::I32(0));
                }
                Some(Literal::I32(l.wrapping_rem(r)))
            }
            (BinaryOp::RemUInt32, Literal::I32(l), Literal::I32(r)) => {
                if r == 0 {
                    return None;
                }
                Some(Literal::I32((l as u32).wrapping_rem(r as u32) as i32))
            }
            (BinaryOp::AndInt32, Literal::I32(l), Literal::I32(r)) => Some(Literal::I32(l & r)),
            (BinaryOp::OrInt32, Literal::I32(l), Literal::I32(r)) => Some(Literal::I32(l | r)),
            (BinaryOp::XorInt32, Literal::I32(l), Literal::I32(r)) => Some(Literal::I32(l ^ r)),
            (BinaryOp::ShlInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(l.wrapping_shl(r as u32)))
            }
            (BinaryOp::ShrSInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(l.wrapping_shr(r as u32)))
            }
            (BinaryOp::ShrUInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32((l as u32).wrapping_shr(r as u32) as i32))
            }
            (BinaryOp::RotLInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(l.rotate_left(r as u32)))
            }
            (BinaryOp::RotRInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(l.rotate_right(r as u32)))
            }
            (BinaryOp::EqInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if l == r { 1 } else { 0 }))
            }
            (BinaryOp::NeInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if l != r { 1 } else { 0 }))
            }
            (BinaryOp::LtSInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if l < r { 1 } else { 0 }))
            }
            (BinaryOp::LtUInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if (l as u32) < (r as u32) { 1 } else { 0 }))
            }
            (BinaryOp::LeSInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if l <= r { 1 } else { 0 }))
            }
            (BinaryOp::LeUInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if (l as u32) <= (r as u32) { 1 } else { 0 }))
            }
            (BinaryOp::GtSInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if l > r { 1 } else { 0 }))
            }
            (BinaryOp::GtUInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if (l as u32) > (r as u32) { 1 } else { 0 }))
            }
            (BinaryOp::GeSInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if l >= r { 1 } else { 0 }))
            }
            (BinaryOp::GeUInt32, Literal::I32(l), Literal::I32(r)) => {
                Some(Literal::I32(if (l as u32) >= (r as u32) { 1 } else { 0 }))
            }

            // TODO: Implement i64, f32, f64 ops
            _ => None,
        }
    }

    fn eval_unary(&self, op: UnaryOp, v: Literal) -> Option<Literal> {
        match (op, v) {
            (UnaryOp::EqZInt32, Literal::I32(v)) => Some(Literal::I32(if v == 0 { 1 } else { 0 })),
            (UnaryOp::ClzInt32, Literal::I32(v)) => Some(Literal::I32(v.leading_zeros() as i32)),
            (UnaryOp::CtzInt32, Literal::I32(v)) => Some(Literal::I32(v.trailing_zeros() as i32)),
            (UnaryOp::PopcntInt32, Literal::I32(v)) => Some(Literal::I32(v.count_ones() as i32)),

            // TODO: Implement i64, f32, f64 ops
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind, IrBuilder};
    use binaryen_core::Type;

    #[test]
    fn test_eval_basic() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let evaluator = Evaluator::new(&bump);

        // 1 + 2 = 3
        let c1 = builder.const_(Literal::I32(1));
        let c2 = builder.const_(Literal::I32(2));
        let add = builder.binary(BinaryOp::AddInt32, c1, c2, Type::I32);

        assert_eq!(evaluator.eval(add), Some(Literal::I32(3)));
    }

    #[test]
    fn test_eval_nested() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let evaluator = Evaluator::new(&bump);

        // (1 + 2) * 3 = 9
        let c1 = builder.const_(Literal::I32(1));
        let c2 = builder.const_(Literal::I32(2));
        let add = builder.binary(BinaryOp::AddInt32, c1, c2, Type::I32);
        let c3 = builder.const_(Literal::I32(3));
        let mul = builder.binary(BinaryOp::MulInt32, add, c3, Type::I32);

        assert_eq!(evaluator.eval(mul), Some(Literal::I32(9)));
    }

    #[test]
    fn test_eval_div_zero() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let evaluator = Evaluator::new(&bump);

        // 1 / 0 = None
        let c1 = builder.const_(Literal::I32(1));
        let c0 = builder.const_(Literal::I32(0));
        let div = builder.binary(BinaryOp::DivSInt32, c1, c0, Type::I32);

        assert_eq!(evaluator.eval(div), None);
    }
}
