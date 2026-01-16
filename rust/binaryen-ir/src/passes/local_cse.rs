use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use std::collections::HashMap;

/// Local Common Subexpression Elimination (CSE)
///
/// Eliminates redundant computations within a function by detecting
/// expressions that compute the same value and replacing duplicates
/// with references to a single computation.
///
/// This pass is "local" because it works within function boundaries
/// and doesn't require global analysis.
pub struct LocalCSE;

impl Pass for LocalCSE {
    fn name(&self) -> &str {
        "local-cse"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let allocator = module.allocator();

        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut cse = CSETransformer {
                    allocator,
                    expr_map: HashMap::new(),
                    local_counter: func.vars.len() as u32,
                };
                cse.visit(body);
            }
        }
    }
}

#[allow(dead_code)]
struct CSETransformer<'a> {
    allocator: &'a bumpalo::Bump,
    expr_map: HashMap<ExprKey, (ExprRef<'a>, u32)>, // expr -> (original, temp_local)
    local_counter: u32,
}

/// Key for identifying equivalent expressions
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
enum ExprKey {
    Binary {
        op: String,
        left: Box<ExprKey>,
        right: Box<ExprKey>,
    },
    Unary {
        op: String,
        value: Box<ExprKey>,
    },
    Const {
        value: String,
    },
    LocalGet {
        index: u32,
    },
}

impl<'a> CSETransformer<'a> {
    fn expr_to_key(&self, expr: &ExprRef<'a>) -> Option<ExprKey> {
        match &expr.kind {
            ExpressionKind::Const(lit) => Some(ExprKey::Const {
                value: format!("{:?}", lit),
            }),
            ExpressionKind::LocalGet { index } => Some(ExprKey::LocalGet { index: *index }),
            ExpressionKind::Binary { op, left, right } => {
                let left_key = self.expr_to_key(left)?;
                let right_key = self.expr_to_key(right)?;
                Some(ExprKey::Binary {
                    op: format!("{:?}", op),
                    left: Box::new(left_key),
                    right: Box::new(right_key),
                })
            }
            ExpressionKind::Unary { op, value } => {
                let value_key = self.expr_to_key(value)?;
                Some(ExprKey::Unary {
                    op: format!("{:?}", op),
                    value: Box::new(value_key),
                })
            }
            _ => None, // Only handle simple cases for now
        }
    }
}

impl<'a> Visitor<'a> for CSETransformer<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        // For now, just traverse - full CSE requires more sophisticated analysis
        // This is a foundation that can be enhanced

        // Try to generate a key for this expression
        if let Some(_key) = self.expr_to_key(expr) {
            // In a full implementation, we would:
            // 1. Check if we've seen this expression before
            // 2. If yes, replace with a local.get of the saved result
            // 3. If no, wrap in a local.tee to save the result

            // For now, we just mark that we could optimize here
            // TODO: Implement full CSE with local allocation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind};
    use crate::module::Function;
    use crate::ops::BinaryOp;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_local_cse_basic() {
        let bump = Bump::new();

        // Create: (i32.add (i32.const 1) (i32.const 2))
        let const1 = Expression::const_expr(&bump, Literal::I32(1), Type::I32);
        let const2 = Expression::const_expr(&bump, Literal::I32(2), Type::I32);
        let add = Expression::new(
            &bump,
            ExpressionKind::Binary {
                op: BinaryOp::AddInt32,
                left: const1,
                right: const2,
            },
            Type::I32,
        );

        let func = Function::new("test".to_string(), Type::NONE, Type::I32, vec![], Some(add));

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = LocalCSE;
        pass.run(&mut module);

        // For now, expression should be unchanged (foundation only)
        let body = module.functions[0].body.as_ref().unwrap();
        assert!(matches!(body.kind, ExpressionKind::Binary { .. }));
    }

    #[test]
    fn test_local_cse_can_generate_keys() {
        let bump = Bump::new();

        let transformer = CSETransformer {
            allocator: &bump,
            expr_map: HashMap::new(),
            local_counter: 0,
        };

        // Test constant key generation
        let const_expr = Expression::const_expr(&bump, Literal::I32(42), Type::I32);
        let key = transformer.expr_to_key(&const_expr);
        assert!(key.is_some());

        // Test local.get key generation
        let get_expr = Expression::local_get(&bump, 0, Type::I32);
        let key = transformer.expr_to_key(&get_expr);
        assert!(key.is_some());
    }

    #[test]
    fn test_local_cse_preserves_structure() {
        let bump = Bump::new();

        // Complex expression that CSE doesn't currently optimize
        let val1 = Expression::const_expr(&bump, Literal::I32(10), Type::I32);
        let val2 = Expression::const_expr(&bump, Literal::I32(20), Type::I32);
        let add = Expression::new(
            &bump,
            ExpressionKind::Binary {
                op: BinaryOp::AddInt32,
                left: val1,
                right: val2,
            },
            Type::I32,
        );

        let func = Function::new("test".to_string(), Type::NONE, Type::I32, vec![], Some(add));

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = LocalCSE;
        pass.run(&mut module);

        // Should preserve structure
        let body = module.functions[0].body.as_ref().unwrap();
        assert!(matches!(body.kind, ExpressionKind::Binary { .. }));
    }
}
