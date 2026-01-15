use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::ops::BinaryOp;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::Literal;

pub struct SimplifyIdentity;

impl Pass for SimplifyIdentity {
    fn name(&self) -> &str {
        "SimplifyIdentity"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                self.visit(body);
            }
        }
    }
}

impl<'a> Visitor<'a> for SimplifyIdentity {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        // Optimization: x + 0 -> x
        if let ExpressionKind::Binary {
            op,
            left: _left,
            right,
        } = &mut expr.kind
        {
            let is_identity = match op {
                BinaryOp::AddInt32 | BinaryOp::SubInt32 => {
                    matches!(right.kind, ExpressionKind::Const(Literal::I32(0)))
                }
                BinaryOp::AddInt64 | BinaryOp::SubInt64 => {
                    matches!(right.kind, ExpressionKind::Const(Literal::I64(0)))
                }
                BinaryOp::MulInt32 => {
                    matches!(right.kind, ExpressionKind::Const(Literal::I32(1)))
                }
                BinaryOp::MulInt64 => {
                    matches!(right.kind, ExpressionKind::Const(Literal::I64(1)))
                }
                _ => false,
            };

            if is_identity {
                // Perform the replacement: *expr = *left
                // 1. Take ownership of the inner binary expression kind temporarily
                let mut kind = std::mem::replace(&mut expr.kind, ExpressionKind::Nop);

                // 2. Extract the left child
                if let ExpressionKind::Binary { left, .. } = &mut kind {
                    // 3. Move left's content into expr
                    // We need to swap left.kind into expr.kind and left.type_ into expr.type_
                    expr.type_ = left.type_;
                    expr.kind = std::mem::replace(&mut left.kind, ExpressionKind::Nop);
                } else {
                    unreachable!("We just matched Binary");
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
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_simplify_identity_add_zero() {
        let bump = Bump::new();

        // Construct: (val + 0)
        // Expected: val

        let val = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        }));

        let zero = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(0)),
            type_: Type::I32,
        }));

        let binary = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Binary {
                op: BinaryOp::AddInt32,
                left: val,
                right: zero,
            },
            type_: Type::I32,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(binary),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = SimplifyIdentity;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();

        // Body should now be just Const(42)
        match body.kind {
            ExpressionKind::Const(Literal::I32(42)) => {} // OK
            ExpressionKind::Binary { .. } => panic!("Optimization failed, binary op still present"),
            _ => panic!("Unexpected expression kind: {:?}", body.kind),
        }
    }
}
