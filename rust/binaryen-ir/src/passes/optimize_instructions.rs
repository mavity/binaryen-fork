use crate::analysis::patterns::{Env, Pattern, PatternMatcher};
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
        Self::register_algebraic_identities(&mut matcher);
        Self::register_strength_reduction(&mut matcher);
        Self::register_comparison_optimizations(&mut matcher);
        Self { matcher }
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
            |env: &Env, bump| {
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
            |env: &Env, bump| {
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
            |env: &Env, bump| {
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
            |env: &Env, bump| {
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
            |env: &Env, _| env.get("x").copied(),
        );

        // 0 + x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AddInt32,
                Pattern::Const(Literal::I32(0)),
                Pattern::Var("x"),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // --- MulInt32 ---

        // x * 1 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::MulInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(1)),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // 1 * x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::MulInt32,
                Pattern::Const(Literal::I32(1)),
                Pattern::Var("x"),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // --- SubInt32 ---

        // x - 0 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::SubInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // --- AndInt32 ---

        // x & -1 -> x (Identity)
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AndInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(-1)),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // -1 & x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AndInt32,
                Pattern::Const(Literal::I32(-1)),
                Pattern::Var("x"),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // --- OrInt32 ---

        // x | 0 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::OrInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // 0 | x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::OrInt32,
                Pattern::Const(Literal::I32(0)),
                Pattern::Var("x"),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // --- XorInt32 ---

        // x ^ 0 -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::XorInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0)),
            ),
            |env: &Env, _| env.get("x").copied(),
        );

        // 0 ^ x -> x
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::XorInt32,
                Pattern::Const(Literal::I32(0)),
                Pattern::Var("x"),
            ),
            |env: &Env, _| env.get("x").copied(),
        );
    }

    fn register_strength_reduction(matcher: &mut PatternMatcher) {
        // x * 2^k -> x << k
        matcher.add_rule(
            Pattern::binary(BinaryOp::MulInt32, Pattern::Var("x"), Pattern::Var("c")),
            |env: &Env, bump| {
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
            |env: &Env, bump| {
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
            |env: &Env, bump| {
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
}
