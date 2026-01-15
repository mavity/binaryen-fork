use crate::analysis::patterns::{Pattern, PatternMatcher, Env};
use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::ops::BinaryOp;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::Literal;

/// OptimizeInstructions pass: Algebraic simplifications and strength reduction
pub struct OptimizeInstructions {
    matcher: PatternMatcher,
}

impl OptimizeInstructions {
    pub fn new() -> Self {
        let mut matcher = PatternMatcher::new();
        Self::register_algebraic_identities(&mut matcher);
        Self { matcher }
    }

    fn register_algebraic_identities(matcher: &mut PatternMatcher) {
        // --- AddInt32 ---
        
        // x + 0 -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::AddInt32, Pattern::Var("x"), Pattern::Const(Literal::I32(0))),
            |env| env.get("x").copied()
        );

        // 0 + x -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::AddInt32, Pattern::Const(Literal::I32(0)), Pattern::Var("x")),
            |env| env.get("x").copied()
        );

        // --- MulInt32 ---

        // x * 1 -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::MulInt32, Pattern::Var("x"), Pattern::Const(Literal::I32(1))),
            |env| env.get("x").copied()
        );

        // 1 * x -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::MulInt32, Pattern::Const(Literal::I32(1)), Pattern::Var("x")),
            |env| env.get("x").copied()
        );
        
        // x * 0 -> 0 (Note: x must be side-effect free? For now we assume strict or allow reordering if it's safe-ish. 
        // In Binaryen C++, x * 0 -> 0 is done if x is side effect free. 
        // We will skip x * 0 for now until we have side-effect analysis in the pattern matcher)

        // --- SubInt32 ---

        // x - 0 -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::SubInt32, Pattern::Var("x"), Pattern::Const(Literal::I32(0))),
            |env| env.get("x").copied()
        );

        // --- AndInt32 ---
        
        // x & 0 -> 0
        // Similar to x * 0, this drops x. Skip for now.
        
        // x & -1 -> x (Identity)
        matcher.add_rule(
            Pattern::binary(BinaryOp::AndInt32, Pattern::Var("x"), Pattern::Const(Literal::I32(-1))),
            |env| env.get("x").copied()
        );
        
        // -1 & x -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::AndInt32, Pattern::Const(Literal::I32(-1)), Pattern::Var("x")),
            |env| env.get("x").copied()
        );

        // --- OrInt32 ---
        
        // x | 0 -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::OrInt32, Pattern::Var("x"), Pattern::Const(Literal::I32(0))),
            |env| env.get("x").copied()
        );

        // 0 | x -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::OrInt32, Pattern::Const(Literal::I32(0)), Pattern::Var("x")),
            |env| env.get("x").copied()
        );

        // --- XorInt32 ---

        // x ^ 0 -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::XorInt32, Pattern::Var("x"), Pattern::Const(Literal::I32(0))),
            |env| env.get("x").copied()
        );

        // 0 ^ x -> x
        matcher.add_rule(
            Pattern::binary(BinaryOp::XorInt32, Pattern::Const(Literal::I32(0)), Pattern::Var("x")),
            |env| env.get("x").copied()
        );
    }
}

impl Pass for OptimizeInstructions {
    fn name(&self) -> &str {
        "optimize-instructions"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                self.visit(body);
            }
        }
    }
}

impl<'a> Visitor<'a> for OptimizeInstructions {
    fn visit(&mut self, expr: &mut ExprRef<'a>) {
        // Bottom-up traversal: simplify children first
        self.visit_children(expr);

        // Try to simplify current expression
        if let Some(new_expr) = self.matcher.simplify(*expr) {
            *expr = new_expr;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExprRef, Expression, ExpressionKind, IrBuilder};
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_optimize_add_zero() {
        let bump = Bump::new();
        let mut builder = IrBuilder::new(&bump);

        // (local.get 0) + 0
        let x = builder.local_get(0, Type::I32);
        let zero = builder.const_(Literal::I32(0));
        let add = builder.binary(BinaryOp::AddInt32, x, zero, Type::I32);

        let mut pass = OptimizeInstructions::new();
        // Since we can't easily run full pass without module, we can test visitor logic manually 
        // or construct a dummy module.
        // Let's use visitor logic directly on the expression.
        
        let mut expr_ref = add;
        pass.visit(&mut expr_ref);
        
        // Should be replaced by local.get 0
        assert!(matches!(expr_ref.kind, ExpressionKind::LocalGet { index: 0, .. }));
    }

    #[test]
    fn test_optimize_mul_one() {
        let bump = Bump::new();
        let mut builder = IrBuilder::new(&bump);

        // 1 * (local.get 0)
        let x = builder.local_get(0, Type::I32);
        let one = builder.const_(Literal::I32(1));
        let mul = builder.binary(BinaryOp::MulInt32, one, x, Type::I32);

        let mut pass = OptimizeInstructions::new();
        let mut expr_ref = mul;
        pass.visit(&mut expr_ref);

        assert!(matches!(expr_ref.kind, ExpressionKind::LocalGet { index: 0, .. }));
    }
    
    #[test]
    fn test_optimize_nested() {
        // ((x + 0) * 1) -> x
        let bump = Bump::new();
        let mut builder = IrBuilder::new(&bump);

        let x = builder.local_get(0, Type::I32);
        let zero = builder.const_(Literal::I32(0));
        let add = builder.binary(BinaryOp::AddInt32, x, zero, Type::I32);
        
        let one = builder.const_(Literal::I32(1));
        let mul = builder.binary(BinaryOp::MulInt32, add, one, Type::I32);

        let mut pass = OptimizeInstructions::new();
        let mut expr_ref = mul;
        pass.visit(&mut expr_ref);

        assert!(matches!(expr_ref.kind, ExpressionKind::LocalGet { index: 0, .. }));
    }
}
