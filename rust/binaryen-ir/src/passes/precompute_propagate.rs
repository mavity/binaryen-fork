use crate::analysis::evaluator::Evaluator;
use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::{Literal, Type};
use std::collections::HashMap;

/// PrecomputePropagate pass: Evaluates expressions and propagates constants through locals
pub struct PrecomputePropagate;

impl Pass for PrecomputePropagate {
    fn name(&self) -> &str {
        "precompute-propagate"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let evaluator = Evaluator::new(module.allocator);

        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                // We need to run this until convergence or a fixed number of times
                // since propagating a constant might enable further folding
                for _ in 0..3 {
                    let mut visitor = PrecomputePropagateVisitor::new(&evaluator);
                    visitor.visit(body);
                    if !visitor.made_changes {
                        break;
                    }
                }
            }
        }
    }
}

struct PrecomputePropagateVisitor<'a, 'b> {
    evaluator: &'b Evaluator<'a>,
    /// Known values for locals in the current control flow scope
    /// Map from Local Index -> Literal
    /// Note: This is a very simplified version that doesn't handle control flow merges properly yet.
    /// For a robust implementation, we need dataflow analysis.
    /// For this iteration, we will only propagate within straight-line code and invalidate on control flow.
    known_locals: HashMap<u32, Literal>,
    made_changes: bool,
}

impl<'a, 'b> PrecomputePropagateVisitor<'a, 'b> {
    fn new(evaluator: &'b Evaluator<'a>) -> Self {
        Self {
            evaluator,
            known_locals: HashMap::new(),
            made_changes: false,
        }
    }

    fn invalidate_all(&mut self) {
        self.known_locals.clear();
    }
}

impl<'a, 'b> Visitor<'a> for PrecomputePropagateVisitor<'a, 'b> {
    fn visit(&mut self, expr: &mut ExprRef<'a>) {
        // Pre-order traversal for control flow handling
        match &expr.kind {
            // Control flow structures invalidate our simple straight-line knowledge
            ExpressionKind::Block { .. }
            | ExpressionKind::If { .. }
            | ExpressionKind::Loop { .. }
            | ExpressionKind::Break { .. }
            | ExpressionKind::Switch { .. }
            | ExpressionKind::Call { .. }
            | ExpressionKind::CallIndirect { .. } => {
                // Conservative approach: invalidate everything when entering control flow
                // Real implementation needs proper merging
                self.invalidate_all();
            }
            _ => {}
        }

        // Visit children first (bottom-up for expression folding)
        self.visit_children(expr);

        // Post-order processing
        match &expr.kind {
            ExpressionKind::LocalSet { index, value } => {
                // If value is a constant, record it
                if let ExpressionKind::Const(lit) = &value.kind {
                    self.known_locals.insert(*index, lit.clone());
                } else {
                    // Otherwise, we don't know this local anymore
                    self.known_locals.remove(index);
                }
            }
            ExpressionKind::LocalGet { index, .. } => {
                // If we know the value, replace with constant
                if let Some(lit) = self.known_locals.get(index) {
                    expr.kind = ExpressionKind::Const(lit.clone());
                    self.made_changes = true;
                }
            }
            ExpressionKind::LocalTee { index, value } => {
                // Like Set, record if constant
                if let ExpressionKind::Const(lit) = &value.kind {
                    self.known_locals.insert(*index, lit.clone());
                } else {
                    self.known_locals.remove(index);
                }
            }
            _ => {
                // Try to evaluate constant expressions
                // This reuses the logic from Precompute pass
                if let Some(lit) = self.evaluator.eval(*expr) {
                    if !matches!(expr.kind, ExpressionKind::Const(_)) {
                        expr.kind = ExpressionKind::Const(lit);
                        self.made_changes = true;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExprRef, Expression, ExpressionKind};
    use crate::module::Function;
    use crate::ops::BinaryOp;
    use binaryen_core::{Literal, Type};
    use bumpalo::collections::Vec as BumpVec;
    use bumpalo::Bump;

    #[test]
    fn test_propagate_basic() {
        let bump = Bump::new();
        let builder = crate::expression::IrBuilder::new(&bump);

        // (local.set 0 (i32.const 42))
        // (i32.add (local.get 0) (i32.const 10))

        let const42 = builder.const_(Literal::I32(42));
        let set = builder.local_set(0, const42);

        let get = builder.local_get(0, Type::I32);
        let const10 = builder.const_(Literal::I32(10));
        let add = builder.binary(BinaryOp::AddInt32, get, const10, Type::I32);

        let mut list = BumpVec::new_in(&bump);
        list.push(set);
        list.push(add);
        let body = builder.block(None, list, Type::I32);

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![Type::I32],
            Some(body),
        );

        let bump_mod = bumpalo::Bump::new();
        let mut module = Module::new(&bump_mod);
        module.add_function(func);

        let mut pass = PrecomputePropagate;
        pass.run(&mut module);

        // Check if the add was folded to 52
        let func = &module.functions[0];
        let body = func.body.as_ref().unwrap();

        if let ExpressionKind::Block { list, .. } = &body.kind {
            let last = list.last().unwrap();
            assert!(matches!(last.kind, ExpressionKind::Const(Literal::I32(52))));
        } else {
            panic!("Expected block");
        }
    }
}
