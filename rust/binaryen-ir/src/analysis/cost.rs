use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Function;
use crate::visitor::ReadOnlyVisitor;

/// Cost Estimator for inlining decisions
pub struct CostEstimator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    pub instruction_count: u32,
    pub call_count: u32,
    pub loop_count: u32,
    pub has_try_delegate: bool,
}

impl CostEstimator {
    /// Estimate cost of inlining this function
    pub fn inline_cost(func: &Function) -> Cost {
        let mut calculator = CostCalculator::new();
        if let Some(body) = &func.body {
            calculator.visit(*body);
        }
        calculator.cost
    }
}

struct CostCalculator {
    cost: Cost,
}

impl CostCalculator {
    fn new() -> Self {
        Self {
            cost: Cost {
                instruction_count: 0,
                call_count: 0,
                loop_count: 0,
                has_try_delegate: false,
            },
        }
    }
}

impl<'a> ReadOnlyVisitor<'a> for CostCalculator {
    fn visit_expression(&mut self, expr: ExprRef<'a>) {
        self.cost.instruction_count += 1;

        match &expr.kind {
            ExpressionKind::Call { .. } | ExpressionKind::CallIndirect { .. } => {
                self.cost.call_count += 1;
            }
            ExpressionKind::Loop { .. } => {
                self.cost.loop_count += 1;
            }
            ExpressionKind::Try { delegate, .. } => {
                if delegate.is_some() {
                    self.cost.has_try_delegate = true;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::IrBuilder;
    use binaryen_core::Type;
    use bumpalo::Bump;

    #[test]
    fn test_cost_estimation() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (block
        //   (nop)
        //   (loop (nop))
        //   (call "foo")
        // )
        // Count: Block(1) + Nop(1) + Loop(1) + Nop(1) + Call(1) = 5 instrs
        // Loop count: 1
        // Call count: 1

        let nop1 = builder.nop();
        let nop2 = builder.nop();
        let loop_ = builder.loop_(None, nop2, Type::NONE);
        let call = builder.call(
            "foo",
            bumpalo::collections::Vec::new_in(&bump),
            Type::NONE,
            false,
        );

        let mut list = bumpalo::collections::Vec::new_in(&bump);
        list.push(nop1);
        list.push(loop_);
        list.push(call);

        let block = builder.block(None, list, Type::NONE);

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let cost = CostEstimator::inline_cost(&func);

        assert_eq!(cost.instruction_count, 5);
        assert_eq!(cost.loop_count, 1);
        assert_eq!(cost.call_count, 1);
    }
}
