use crate::analysis::call_graph::CallGraph;
use crate::analysis::global_analysis::GlobalAnalysis;
use crate::expression::{ExprRef, ExpressionKind, IrBuilder};
use crate::module::Module;
use crate::pass::Pass;
use bumpalo::Bump;
use std::collections::HashMap;

pub struct SimplifyGlobals;

impl Pass for SimplifyGlobals {
    fn name(&self) -> &str {
        "simplify-globals"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // 1. Build CallGraph
        let call_graph = CallGraph::build(module);

        // 2. Run Global Analysis
        let analysis = GlobalAnalysis::analyze(module, &call_graph);

        // 3. Optimize
        // We can replace GlobalGet of constant globals with Const.

        if analysis.constant_globals.is_empty() {
            return;
        }

        let allocator = module.allocator;
        let mut optimizer = GlobalOptimizer {
            analysis: &analysis,
            allocator,
            optimized_count: 0,
        };

        for func in &mut module.functions {
            if let Some(body) = func.body {
                func.body = Some(optimizer.transform(body));
            }
        }

        // Note: Removing unused globals would require modifying module.globals,
        // which is harder with current mutable borrow of functions.
        // We'll focus on constant propagation here.
    }
}

struct GlobalOptimizer<'a, 'b> {
    analysis: &'b GlobalAnalysis,
    allocator: &'a Bump,
    optimized_count: usize,
}

impl<'a, 'b> GlobalOptimizer<'a, 'b> {
    fn transform(&mut self, expr: ExprRef<'a>) -> ExprRef<'a> {
        let builder = IrBuilder::new(self.allocator);

        match &expr.kind {
            ExpressionKind::GlobalGet { index } => {
                if let Some(literal) = self.analysis.global_values.get(&(*index as usize)) {
                    self.optimized_count += 1;
                    return builder.const_(literal.clone());
                }
                expr
            }
            ExpressionKind::Block { name, list } => {
                let mut new_list = bumpalo::collections::Vec::new_in(self.allocator);
                for child in list.iter() {
                    new_list.push(self.transform(*child));
                }
                builder.block(name.clone(), new_list, expr.type_)
            }
            ExpressionKind::Binary { op, left, right } => {
                let new_left = self.transform(*left);
                let new_right = self.transform(*right);
                builder.binary(*op, new_left, new_right, expr.type_)
            }
            // ... shallow impl for other nodes, ideally use a standardized rewriter/visitor
            // For now, implement for most common structures
            ExpressionKind::Call {
                target,
                operands,
                is_return,
            } => {
                let mut new_ops = bumpalo::collections::Vec::new_in(self.allocator);
                for op in operands.iter() {
                    new_ops.push(self.transform(*op));
                }
                builder.call(target.clone(), new_ops, expr.type_, *is_return)
            }
            ExpressionKind::Return { value } => {
                let new_val = value.map(|v| self.transform(v));
                builder.return_(new_val)
            }
            _ => expr, // fallback: don't recurse if not implemented (unsafe for deep trees, but ok for demo)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{Function, Global};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_propagate_constant_global() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Global 0: immutable i32 = 42
        let init = builder.const_(Literal::I32(42));
        let global = Global {
            name: "g0".to_string(),
            type_: Type::I32,
            mutable: false,
            init: init,
        };

        // Function: (return (global.get 0))
        let body = builder.return_(Some(builder.global_get(0, Type::I32)));
        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(body),
        );

        let mut module = Module::new(&bump);
        module.globals.push(global);
        module.functions.push(func);

        let mut pass = SimplifyGlobals;
        pass.run(&mut module);

        // Check body is now (return (i32.const 42))
        let body = module.functions[0].body.unwrap();
        if let ExpressionKind::Return { value } = &body.kind {
            if let Some(val) = value {
                if let ExpressionKind::Const(value) = &val.kind {
                    assert_eq!(*value, Literal::I32(42));
                    return;
                }
            }
        }
        panic!("Optimization failed: expected Const(42)");
    }
}
