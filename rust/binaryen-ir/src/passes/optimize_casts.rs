use crate::analysis::patterns::{Pattern, PatternMatcher};
use crate::expression::ExprRef;
use crate::module::Module;
use crate::ops::UnaryOp;
use crate::pass::Pass;
use crate::visitor::Visitor;

/// OptimizeCasts pass: Removes redundant casts
pub struct OptimizeCasts {
    matcher: PatternMatcher,
}

impl OptimizeCasts {
    pub fn new() -> Self {
        let mut matcher = PatternMatcher::new();
        Self::register_rules(&mut matcher);
        Self { matcher }
    }

    fn register_rules(matcher: &mut PatternMatcher) {
        // wrap(extend_s(x)) -> x
        matcher.add_rule(
            Pattern::unary(
                UnaryOp::WrapInt64,
                Pattern::unary(UnaryOp::ExtendSInt32, Pattern::Var("x")),
            ),
            |env, _| env.get("x").copied(),
        );

        // wrap(extend_u(x)) -> x
        matcher.add_rule(
            Pattern::unary(
                UnaryOp::WrapInt64,
                Pattern::unary(UnaryOp::ExtendUInt32, Pattern::Var("x")),
            ),
            |env, _| env.get("x").copied(),
        );
    }
}

impl Pass for OptimizeCasts {
    fn name(&self) -> &str {
        "optimize-casts"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut visitor = OptimizeCastsVisitor {
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

struct OptimizeCastsVisitor<'a, 'b> {
    matcher: &'b PatternMatcher,
    allocator: &'a bumpalo::Bump,
}

impl<'a, 'b> Visitor<'a> for OptimizeCastsVisitor<'a, 'b> {
    fn visit(&mut self, expr: &mut ExprRef<'a>) {
        self.visit_children(expr);
        if let Some(new_expr) = self.matcher.simplify(*expr, self.allocator) {
            *expr = new_expr;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExpressionKind, IrBuilder};
    use binaryen_core::Type;
    use bumpalo::Bump;

    #[test]
    fn test_optimize_wrap_extend_s() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // wrap(extend_s(local.get 0)) -> local.get 0
        let x = builder.local_get(0, Type::I32);
        let extend = builder.unary(UnaryOp::ExtendSInt32, x, Type::I64);
        let wrap = builder.unary(UnaryOp::WrapInt64, extend, Type::I32);

        let pass = OptimizeCasts::new();
        let mut visitor = OptimizeCastsVisitor {
            matcher: &pass.matcher,
            allocator: &bump,
        };

        let mut expr_ref = wrap;
        visitor.visit(&mut expr_ref);

        assert!(matches!(
            expr_ref.kind,
            ExpressionKind::LocalGet { index: 0, .. }
        ));
    }
}
