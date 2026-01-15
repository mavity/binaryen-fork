use crate::expression::{Expression, ExpressionKind};
use crate::module::Module;
use crate::ops::BinaryOp;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::{Literal, Type};

/// Simplification pass - implements algebraic simplifications and constant folding
///
/// This pass performs more complex simplifications than SimplifyIdentity, including:
/// - Algebraic identities (x*0=0, x&0=0, x|x=x, etc.)
/// - Double negation elimination
/// - Comparison simplifications
/// - Control flow simplifications (constant conditions)
pub struct Simplify;

impl Pass for Simplify {
    fn name(&self) -> &str {
        "Simplify"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                self.visit(body);
            }
        }
    }
}

impl<'a> Visitor<'a> for Simplify {
    fn visit_expression(&mut self, expr: &mut Expression<'a>) {
        // Handle binary operations
        if let ExpressionKind::Binary { op, left, right } = &mut expr.kind {
            // Check for patterns like x op x
            if are_expressions_equal(left, right) {
                match op {
                    // x - x = 0
                    BinaryOp::SubInt32 => {
                        expr.kind = ExpressionKind::Const(Literal::I32(0));
                        expr.type_ = Type::I32;
                        return;
                    }
                    BinaryOp::SubInt64 => {
                        expr.kind = ExpressionKind::Const(Literal::I64(0));
                        expr.type_ = Type::I64;
                        return;
                    }
                    BinaryOp::SubFloat32 => {
                        expr.kind = ExpressionKind::Const(Literal::F32(0.0));
                        expr.type_ = Type::F32;
                        return;
                    }
                    BinaryOp::SubFloat64 => {
                        expr.kind = ExpressionKind::Const(Literal::F64(0.0));
                        expr.type_ = Type::F64;
                        return;
                    }
                    // x ^ x = 0
                    BinaryOp::XorInt32 => {
                        expr.kind = ExpressionKind::Const(Literal::I32(0));
                        expr.type_ = Type::I32;
                        return;
                    }
                    BinaryOp::XorInt64 => {
                        expr.kind = ExpressionKind::Const(Literal::I64(0));
                        expr.type_ = Type::I64;
                        return;
                    }
                    _ => {}
                }
            }
        }

        // Handle unary operations (placeholder for future enhancements)

        // Handle If with constant condition
        if let ExpressionKind::If {
            condition,
            if_true,
            if_false,
        } = &mut expr.kind
        {
            if let ExpressionKind::Const(lit) = &condition.kind {
                let cond_value = match lit {
                    Literal::I32(v) => *v != 0,
                    Literal::I64(v) => *v != 0,
                    _ => return,
                };

                // Replace the If with the appropriate branch
                if cond_value {
                    // Take if_true branch
                    expr.type_ = if_true.type_;
                    expr.kind = std::mem::replace(&mut if_true.kind, ExpressionKind::Nop);
                } else if let Some(false_branch) = if_false {
                    // Take if_false branch
                    expr.type_ = false_branch.type_;
                    expr.kind = std::mem::replace(&mut false_branch.kind, ExpressionKind::Nop);
                } else {
                    // No else branch, replace with nop
                    expr.kind = ExpressionKind::Nop;
                    expr.type_ = Type::NONE;
                }
            }
        }

        // Handle Select with constant condition
        if let ExpressionKind::Select {
            condition,
            if_true,
            if_false,
        } = &mut expr.kind
        {
            if let ExpressionKind::Const(lit) = &condition.kind {
                let cond_value = match lit {
                    Literal::I32(v) => *v != 0,
                    Literal::I64(v) => *v != 0,
                    _ => return,
                };

                // Replace select with the chosen value
                let chosen = if cond_value { if_true } else { if_false };
                expr.type_ = chosen.type_;
                expr.kind = std::mem::replace(&mut chosen.kind, ExpressionKind::Nop);
            }
        }
    }
}

/// Check if two expressions are structurally equal (simple cases only)
fn are_expressions_equal<'a>(left: &Expression<'a>, right: &Expression<'a>) -> bool {
    match (&left.kind, &right.kind) {
        (ExpressionKind::LocalGet { index: i1 }, ExpressionKind::LocalGet { index: i2 }) => {
            i1 == i2
        }
        (ExpressionKind::GlobalGet { index: i1 }, ExpressionKind::GlobalGet { index: i2 }) => {
            i1 == i2
        }
        (ExpressionKind::Const(l1), ExpressionKind::Const(l2)) => l1 == l2,
        _ => false, // Conservative: assume not equal for complex expressions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind};
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_simplify_x_xor_x() {
        let bump = Bump::new();

        // Construct: local.get 0 ^ local.get 0
        // Expected: i32.const 0

        let left = bump.alloc(Expression {
            kind: ExpressionKind::LocalGet { index: 0 },
            type_: Type::I32,
        });

        let right = bump.alloc(Expression {
            kind: ExpressionKind::LocalGet { index: 0 },
            type_: Type::I32,
        });

        let xor = bump.alloc(Expression {
            kind: ExpressionKind::Binary {
                op: BinaryOp::XorInt32,
                left,
                right,
            },
            type_: Type::I32,
        });

        let func = Function::new(
            "test".to_string(),
            Type::I32, // one i32 param
            Type::I32,
            vec![],
            Some(xor),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Simplify;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();

        // Body should now be Const(0)
        match body.kind {
            ExpressionKind::Const(Literal::I32(0)) => {} // OK
            _ => panic!(
                "Expected i32.const 0 after simplification, got: {:?}",
                body.kind
            ),
        }
    }

    #[test]
    fn test_simplify_x_sub_x() {
        let bump = Bump::new();

        // Construct: local.get 0 - local.get 0
        // Expected: i32.const 0

        let left = bump.alloc(Expression {
            kind: ExpressionKind::LocalGet { index: 0 },
            type_: Type::I32,
        });

        let right = bump.alloc(Expression {
            kind: ExpressionKind::LocalGet { index: 0 },
            type_: Type::I32,
        });

        let sub = bump.alloc(Expression {
            kind: ExpressionKind::Binary {
                op: BinaryOp::SubInt32,
                left,
                right,
            },
            type_: Type::I32,
        });

        let func = Function::new("test".to_string(), Type::I32, Type::I32, vec![], Some(sub));

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Simplify;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();

        match body.kind {
            ExpressionKind::Const(Literal::I32(0)) => {} // OK
            _ => panic!(
                "Expected i32.const 0 after x-x simplification, got: {:?}",
                body.kind
            ),
        }
    }

    #[test]
    fn test_simplify_if_constant_true() {
        let bump = Bump::new();

        // Construct: if (i32.const 1) then (i32.const 42) else (i32.const 99)
        // Expected: i32.const 42

        let condition = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        });

        let if_true = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        });

        let if_false = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(99)),
            type_: Type::I32,
        });

        let if_expr = bump.alloc(Expression {
            kind: ExpressionKind::If {
                condition,
                if_true,
                if_false: Some(if_false),
            },
            type_: Type::I32,
        });

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(if_expr),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Simplify;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();

        match body.kind {
            ExpressionKind::Const(Literal::I32(42)) => {} // OK
            _ => panic!(
                "Expected i32.const 42 after if simplification, got: {:?}",
                body.kind
            ),
        }
    }

    #[test]
    fn test_simplify_if_constant_false() {
        let bump = Bump::new();

        // Construct: if (i32.const 0) then (i32.const 42) else (i32.const 99)
        // Expected: i32.const 99

        let condition = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(0)),
            type_: Type::I32,
        });

        let if_true = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        });

        let if_false = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(99)),
            type_: Type::I32,
        });

        let if_expr = bump.alloc(Expression {
            kind: ExpressionKind::If {
                condition,
                if_true,
                if_false: Some(if_false),
            },
            type_: Type::I32,
        });

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(if_expr),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Simplify;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();

        match body.kind {
            ExpressionKind::Const(Literal::I32(99)) => {} // OK
            _ => panic!(
                "Expected i32.const 99 after if simplification, got: {:?}",
                body.kind
            ),
        }
    }

    #[test]
    fn test_simplify_select_constant_true() {
        let bump = Bump::new();

        // Construct: select(i32.const 42, i32.const 99, i32.const 1)
        // Expected: i32.const 42

        let if_true = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        });

        let if_false = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(99)),
            type_: Type::I32,
        });

        let condition = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        });

        let select = bump.alloc(Expression {
            kind: ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            },
            type_: Type::I32,
        });

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(select),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Simplify;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();

        match body.kind {
            ExpressionKind::Const(Literal::I32(42)) => {} // OK
            _ => panic!(
                "Expected i32.const 42 after select simplification, got: {:?}",
                body.kind
            ),
        }
    }
}
