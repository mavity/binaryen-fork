use crate::analysis::patterns::{MatchEnv, Pattern, PatternMatcher, PatternOp};
use crate::expression::{ExprRef, ExpressionKind, IrBuilder};
use crate::module::Module;
use crate::ops::BinaryOp;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::{Literal, Type};

/// OptimizeInstructions pass: Algebraic simplifications and strength reduction
pub struct OptimizeInstructions {
    matcher: PatternMatcher,
}

impl Default for OptimizeInstructions {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizeInstructions {
    pub fn new() -> Self {
        let mut matcher = PatternMatcher::new();
        Self::register_constant_folding(&mut matcher);
        Self::register_algebraic_identities(&mut matcher);
        Self::register_strength_reduction(&mut matcher);
        Self::register_comparison_optimizations(&mut matcher);
        Self { matcher }
    }

    fn register_constant_folding(matcher: &mut PatternMatcher) {
        // Fold binary operations on constants.
        matcher.add_rule(
            Pattern::binary(PatternOp::AnyOp, Pattern::AnyConst, Pattern::AnyConst),
            |env, bump| {
                let left = env.get_const("left")?;
                let right = env.get_const("right")?;
                let op = env.get_op()?;

                let builder = IrBuilder::new(bump);

                // It is important to check for the operation because
                // not all binary operations can be folded.
                if let Some(val) = Self::eval_binary_op(op, left, right) {
                    Some(builder.const_(val))
                } else {
                    None
                }
            },
        );
    }

    fn eval_binary_op(op: BinaryOp, left: Literal, right: Literal) -> Option<Literal> {
        use BinaryOp::*;
        match op {
            AddInt32 => Some(Literal::I32(left.get_i32().wrapping_add(right.get_i32()))),
            SubInt32 => Some(Literal::I32(left.get_i32().wrapping_sub(right.get_i32()))),
            MulInt32 => Some(Literal::I32(left.get_i32().wrapping_mul(right.get_i32()))),
            DivSInt32 => {
                let r = right.get_i32();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I32(left.get_i32().wrapping_div(r)))
                }
            }
            DivUInt32 => {
                let r = right.get_u32();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I32((left.get_u32() / r) as i32))
                }
            }
            RemSInt32 => {
                let r = right.get_i32();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I32(left.get_i32().wrapping_rem(r)))
                }
            }
            RemUInt32 => {
                let r = right.get_u32();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I32((left.get_u32() % r) as i32))
                }
            }
            AndInt32 => Some(Literal::I32(left.get_i32() & right.get_i32())),
            OrInt32 => Some(Literal::I32(left.get_i32() | right.get_i32())),
            XorInt32 => Some(Literal::I32(left.get_i32() ^ right.get_i32())),
            ShlInt32 => Some(Literal::I32(left.get_i32().wrapping_shl(right.get_u32()))),
            ShrUInt32 => Some(Literal::I32(
                left.get_u32().wrapping_shr(right.get_u32()) as i32
            )),
            ShrSInt32 => Some(Literal::I32(left.get_i32().wrapping_shr(right.get_u32()))),
            RotLInt32 => Some(Literal::I32(left.get_i32().rotate_left(right.get_u32()))),
            RotRInt32 => Some(Literal::I32(left.get_i32().rotate_right(right.get_u32()))),

            EqInt32 => Some(Literal::I32((left.get_i32() == right.get_i32()) as i32)),
            NeInt32 => Some(Literal::I32((left.get_i32() != right.get_i32()) as i32)),
            LtSInt32 => Some(Literal::I32((left.get_i32() < right.get_i32()) as i32)),
            LtUInt32 => Some(Literal::I32((left.get_u32() < right.get_u32()) as i32)),
            LeSInt32 => Some(Literal::I32((left.get_i32() <= right.get_i32()) as i32)),
            LeUInt32 => Some(Literal::I32((left.get_u32() <= right.get_u32()) as i32)),
            GtSInt32 => Some(Literal::I32((left.get_i32() > right.get_i32()) as i32)),
            GtUInt32 => Some(Literal::I32((left.get_u32() > right.get_u32()) as i32)),
            GeSInt32 => Some(Literal::I32((left.get_i32() >= right.get_i32()) as i32)),
            GeUInt32 => Some(Literal::I32((left.get_u32() >= right.get_u32()) as i32)),

            AddInt64 => Some(Literal::I64(left.get_i64().wrapping_add(right.get_i64()))),
            SubInt64 => Some(Literal::I64(left.get_i64().wrapping_sub(right.get_i64()))),
            MulInt64 => Some(Literal::I64(left.get_i64().wrapping_mul(right.get_i64()))),
            DivSInt64 => {
                let r = right.get_i64();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I64(left.get_i64().wrapping_div(r)))
                }
            }
            DivUInt64 => {
                let r = right.get_u64();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I64((left.get_u64() / r) as i64))
                }
            }
            RemSInt64 => {
                let r = right.get_i64();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I64(left.get_i64().wrapping_rem(r)))
                }
            }
            RemUInt64 => {
                let r = right.get_u64();
                if r == 0 {
                    None // division by zero
                } else {
                    Some(Literal::I64((left.get_u64() % r) as i64))
                }
            }
            AndInt64 => Some(Literal::I64(left.get_i64() & right.get_i64())),
            OrInt64 => Some(Literal::I64(left.get_i64() | right.get_i64())),
            XorInt64 => Some(Literal::I64(left.get_i64() ^ right.get_i64())),
            ShlInt64 => Some(Literal::I64(left.get_i64().wrapping_shl(right.get_u32()))),
            ShrUInt64 => Some(Literal::I64(
                left.get_u64().wrapping_shr(right.get_u32()) as i64
            )),
            ShrSInt64 => Some(Literal::I64(left.get_i64().wrapping_shr(right.get_u32()))),
            RotLInt64 => Some(Literal::I64(left.get_i64().rotate_left(right.get_u32()))),
            RotRInt64 => Some(Literal::I64(left.get_i64().rotate_right(right.get_u32()))),

            EqInt64 => Some(Literal::I32((left.get_i64() == right.get_i64()) as i32)),
            NeInt64 => Some(Literal::I32((left.get_i64() != right.get_i64()) as i32)),
            LtSInt64 => Some(Literal::I32((left.get_i64() < right.get_i64()) as i32)),
            LtUInt64 => Some(Literal::I32((left.get_u64() < right.get_u64()) as i32)),
            LeSInt64 => Some(Literal::I32((left.get_i64() <= right.get_i64()) as i32)),
            LeUInt64 => Some(Literal::I32((left.get_u64() <= right.get_u64()) as i32)),
            GtSInt64 => Some(Literal::I32((left.get_i64() > right.get_i64()) as i32)),
            GtUInt64 => Some(Literal::I32((left.get_u64() > right.get_u64()) as i32)),
            GeSInt64 => Some(Literal::I32((left.get_i64() >= right.get_i64()) as i32)),
            GeUInt64 => Some(Literal::I32((left.get_u64() >= right.get_u64()) as i32)),

            AddFloat32 => Some(Literal::F32(left.get_f32() + right.get_f32())),
            SubFloat32 => Some(Literal::F32(left.get_f32() - right.get_f32())),
            MulFloat32 => Some(Literal::F32(left.get_f32() * right.get_f32())),
            DivFloat32 => Some(Literal::F32(left.get_f32() / right.get_f32())),
            MinFloat32 => Some(Literal::F32(left.get_f32().min(right.get_f32()))),
            MaxFloat32 => Some(Literal::F32(left.get_f32().max(right.get_f32()))),
            CopySignFloat32 => Some(Literal::F32(left.get_f32().copysign(right.get_f32()))),

            EqFloat32 => Some(Literal::I32((left.get_f32() == right.get_f32()) as i32)),
            NeFloat32 => Some(Literal::I32((left.get_f32() != right.get_f32()) as i32)),
            LtFloat32 => Some(Literal::I32((left.get_f32() < right.get_f32()) as i32)),
            LeFloat32 => Some(Literal::I32((left.get_f32() <= right.get_f32()) as i32)),
            GtFloat32 => Some(Literal::I32((left.get_f32() > right.get_f32()) as i32)),
            GeFloat32 => Some(Literal::I32((left.get_f32() >= right.get_f32()) as i32)),

            AddFloat64 => Some(Literal::F64(left.get_f64() + right.get_f64())),
            SubFloat64 => Some(Literal::F64(left.get_f64() - right.get_f64())),
            MulFloat64 => Some(Literal::F64(left.get_f64() * right.get_f64())),
            DivFloat64 => Some(Literal::F64(left.get_f64() / right.get_f64())),
            MinFloat64 => Some(Literal::F64(left.get_f64().min(right.get_f64()))),
            MaxFloat64 => Some(Literal::F64(left.get_f64().max(right.get_f64()))),
            CopySignFloat64 => Some(Literal::F64(left.get_f64().copysign(right.get_f64()))),

            EqFloat64 => Some(Literal::I32((left.get_f64() == right.get_f64()) as i32)),
            NeFloat64 => Some(Literal::I32((left.get_f64() != right.get_f64()) as i32)),
            LtFloat64 => Some(Literal::I32((left.get_f64() < right.get_f64()) as i32)),
            LeFloat64 => Some(Literal::I32((left.get_f64() <= right.get_f64()) as i32)),
            GtFloat64 => Some(Literal::I32((left.get_f64() > right.get_f64()) as i32)),
            GeFloat64 => Some(Literal::I32((left.get_f64() >= right.get_f64()) as i32)),

            _ => None,
        }
    }

    fn register_comparison_optimizations(matcher: &mut PatternMatcher) {
        use crate::ops::UnaryOp;

        // x == 0 -> eqz(x)
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::EqInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &MatchEnv, bump| {
                let x = env.get("x")?;
                let builder = IrBuilder::new(bump);
                Some(builder.unary(UnaryOp::EqZInt32, *x, Type::I32))
            },
        );

        // 0 == x -> eqz(x)
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::EqInt32,
                Pattern::Const(Literal::I32(0)),
                Pattern::Var("x"),
            ),
            |env: &MatchEnv, bump| {
                let x = env.get("x")?;
                let builder = IrBuilder::new(bump);
                Some(builder.unary(UnaryOp::EqZInt32, *x, Type::I32))
            },
        );

        // x >u 0 -> x != 0
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::GtUInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &MatchEnv, bump| {
                let x = env.get("x")?;
                let builder = IrBuilder::new(bump);
                let zero = builder.const_(Literal::I32(0));
                Some(builder.binary(BinaryOp::NeInt32, *x, zero, Type::I32))
            },
        );

        // x <=u 0 -> x == 0 -> eqz(x)
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::LeUInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &MatchEnv, bump| {
                let x = env.get("x")?;
                let builder = IrBuilder::new(bump);
                Some(builder.unary(UnaryOp::EqZInt32, *x, Type::I32))
            },
        );
    }

    fn register_algebraic_identities(matcher: &mut PatternMatcher) {
        // --- AddInt32 ---

        // x + 0 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AddInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // 0 + x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AddInt32,
                Pattern::Const(Literal::I32(0)),
                Pattern::Var("x"),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // --- MulInt32 ---

        // x * 1 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::MulInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(1)),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // 1 * x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::MulInt32,
                Pattern::Const(Literal::I32(1)),
                Pattern::Var("x"),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // --- SubInt32 ---

        // x - 0 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::SubInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // --- AndInt32 ---

        // x & -1 -> x (Identity)
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AndInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(-1)),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // -1 & x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AndInt32,
                Pattern::Const(Literal::I32(-1)),
                Pattern::Var("x"),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // --- OrInt32 ---

        // x | 0 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::OrInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // 0 | x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::OrInt32,
                Pattern::Const(Literal::I32(0)),
                Pattern::Var("x"),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // --- XorInt32 ---

        // x ^ 0 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::XorInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );

        // 0 ^ x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::XorInt32,
                Pattern::Const(Literal::I32(0)),
                Pattern::Var("x"),
            ),
            |env: &MatchEnv, _| env.get("x").copied(),
        );
    }

    fn register_strength_reduction(matcher: &mut PatternMatcher) {
        // x * 2^k -> x << k
        matcher.add_rule(
            Pattern::binary(BinaryOp::MulInt32, Pattern::Var("x"), Pattern::Var("c")),
            |env: &MatchEnv, bump| {
                let x = env.get("x")?;
                let c = env.get("c")?;
                if let ExpressionKind::Const(Literal::I32(val)) = c.kind {
                    if val > 0 && (val & (val - 1)) == 0 {
                        let k = val.trailing_zeros();
                        let builder = IrBuilder::new(bump);
                        let shift_amt = builder.const_(Literal::I32(k as i32));
                        // Clone x because we can't move out of env (shared ref) and can't easily deep clone without more work.
                        // But wait, Expression is in arena. We can just point to it?
                        // IrBuilder takes ExprRef which is Copy.
                        // But x is &ExprRef. *x is ExprRef.
                        return Some(builder.binary(BinaryOp::ShlInt32, *x, shift_amt, Type::I32));
                    }
                }
                None
            },
        );

        // x / 2^k -> x >> k (Unsigned)
        matcher.add_rule(
            Pattern::binary(BinaryOp::DivUInt32, Pattern::Var("x"), Pattern::Var("c")),
            |env: &MatchEnv, bump| {
                let x = env.get("x")?;
                let c = env.get("c")?;
                if let ExpressionKind::Const(Literal::I32(val)) = c.kind {
                    if val > 0 && (val & (val - 1)) == 0 {
                        let k = val.trailing_zeros();
                        let builder = IrBuilder::new(bump);
                        let shift_amt = builder.const_(Literal::I32(k as i32));
                        return Some(builder.binary(BinaryOp::ShrUInt32, *x, shift_amt, Type::I32));
                    }
                }
                None
            },
        );

        // x % 2^k -> x & (2^k - 1) (Unsigned)
        matcher.add_rule(
            Pattern::binary(BinaryOp::RemUInt32, Pattern::Var("x"), Pattern::Var("c")),
            |env: &MatchEnv, bump| {
                let x = env.get("x")?;
                let c = env.get("c")?;
                if let ExpressionKind::Const(Literal::I32(val)) = c.kind {
                    if val > 0 && (val & (val - 1)) == 0 {
                        let mask = val - 1;
                        let builder = IrBuilder::new(bump);
                        let mask_const = builder.const_(Literal::I32(mask));
                        return Some(builder.binary(BinaryOp::AndInt32, *x, mask_const, Type::I32));
                    }
                }
                None
            },
        );
    }
}

impl Pass for OptimizeInstructions {
    fn name(&self) -> &str {
        "optimize-instructions"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &self.matcher,
            allocator: module.allocator,
        };

        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                visitor.visit(body);
            }
        }
    }
}

struct OptimizeInstructionsVisitor<'a, 'b> {
    matcher: &'b PatternMatcher,
    allocator: &'a bumpalo::Bump,
}

impl<'a, 'b> Visitor<'a> for OptimizeInstructionsVisitor<'a, 'b> {
    fn visit(&mut self, expr: &mut ExprRef<'a>) {
        // Bottom-up traversal: simplify children first
        self.visit_children(expr);

        // Try to simplify current expression
        if let Some(new_expr) = self.matcher.simplify(*expr, self.allocator) {
            *expr = new_expr;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExpressionKind, IrBuilder};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_optimize_add_zero() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (local.get 0) + 0
        let x = builder.local_get(0, Type::I32);
        let zero = builder.const_(Literal::I32(0));
        let add = builder.binary(BinaryOp::AddInt32, x, zero, Type::I32);

        let pass = OptimizeInstructions::new();
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = add;
        visitor.visit(&mut expr_ref);

        // Should be replaced by local.get 0
        assert!(matches!(
            expr_ref.kind,
            ExpressionKind::LocalGet { index: 0, .. }
        ));
    }

    #[test]
    fn test_optimize_mul_one() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // 1 * (local.get 0)
        let x = builder.local_get(0, Type::I32);
        let one = builder.const_(Literal::I32(1));
        let mul = builder.binary(BinaryOp::MulInt32, one, x, Type::I32);

        let pass = OptimizeInstructions::new();
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = mul;
        visitor.visit(&mut expr_ref);

        assert!(matches!(
            expr_ref.kind,
            ExpressionKind::LocalGet { index: 0, .. }
        ));
    }

    #[test]
    fn test_optimize_nested() {
        // ((x + 0) * 1) -> x
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let x = builder.local_get(0, Type::I32);
        let zero = builder.const_(Literal::I32(0));
        let add = builder.binary(BinaryOp::AddInt32, x, zero, Type::I32);

        let one = builder.const_(Literal::I32(1));
        let mul = builder.binary(BinaryOp::MulInt32, add, one, Type::I32);

        let pass = OptimizeInstructions::new();
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = mul;
        visitor.visit(&mut expr_ref);

        assert!(matches!(
            expr_ref.kind,
            ExpressionKind::LocalGet { index: 0, .. }
        ));
    }

    #[test]
    fn test_strength_reduction_mul_pow2() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // x * 8 -> x << 3
        let x = builder.local_get(0, Type::I32);
        let eight = builder.const_(Literal::I32(8));
        let mul = builder.binary(BinaryOp::MulInt32, x, eight, Type::I32);

        let pass = OptimizeInstructions::new();
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = mul;
        visitor.visit(&mut expr_ref);

        if let ExpressionKind::Binary { op, right, .. } = expr_ref.kind {
            assert_eq!(op, BinaryOp::ShlInt32);
            if let ExpressionKind::Const(Literal::I32(val)) = right.kind {
                assert_eq!(val, 3);
            } else {
                panic!("Expected constant 3 on RHS");
            }
        } else {
            panic!("Expected shift left");
        }
    }

    use crate::ops::UnaryOp;

    #[test]
    fn test_comparison_eq_zero() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // x == 0 -> eqz(x)
        let x = builder.local_get(0, Type::I32);
        let zero = builder.const_(Literal::I32(0));
        let eq = builder.binary(BinaryOp::EqInt32, x, zero, Type::I32);

        let pass = OptimizeInstructions::new();
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = eq;
        visitor.visit(&mut expr_ref);

        if let ExpressionKind::Unary { op, .. } = expr_ref.kind {
            assert_eq!(op, UnaryOp::EqZInt32);
        } else {
            panic!("Expected EqZ");
        }
    }

    #[test]
    fn test_comparison_gt_u_zero() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // x >u 0 -> x != 0
        let x = builder.local_get(0, Type::I32);
        let zero = builder.const_(Literal::I32(0));
        let gt = builder.binary(BinaryOp::GtUInt32, x, zero, Type::I32);

        let pass = OptimizeInstructions::new();
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = gt;
        visitor.visit(&mut expr_ref);

        if let ExpressionKind::Binary { op, .. } = expr_ref.kind {
            assert_eq!(op, BinaryOp::NeInt32);
        } else {
            panic!("Expected Ne");
        }
    }

    #[test]
    fn test_constant_folding() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // 10 + 20 -> 30
        let ten = builder.const_(Literal::I32(10));
        let twenty = builder.const_(Literal::I32(20));
        let add = builder.binary(BinaryOp::AddInt32, ten, twenty, Type::I32);

        let pass = OptimizeInstructions::new();
        let mut visitor = OptimizeInstructionsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = add;
        visitor.visit(&mut expr_ref);

        if let ExpressionKind::Const(Literal::I32(val)) = expr_ref.kind {
            assert_eq!(val, 30);
        } else {
            panic!("Expected constant 30");
        }
    }
}
