use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::ops::BinaryOp;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::Literal;

/// Precompute pass: Evaluates constant expressions at compile time
pub struct Precompute;

impl Pass for Precompute {
    fn name(&self) -> &str {
        "precompute"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                for _ in 0..3 {
                    self.visit(body);
                }
            }
        }
    }
}

impl<'a> Visitor<'a> for Precompute {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        if let ExpressionKind::Binary { op, left, right } = &expr.kind {
            if let (ExpressionKind::Const(l), ExpressionKind::Const(r)) = (&left.kind, &right.kind)
            {
                if let Some(result) = eval_binary(*op, l, r) {
                    expr.kind = ExpressionKind::Const(result);
                }
            }
        }
    }
}

fn eval_binary(op: BinaryOp, l: &Literal, r: &Literal) -> Option<Literal> {
    match (op, l, r) {
        (BinaryOp::AddInt32, Literal::I32(a), Literal::I32(b)) => {
            Some(Literal::I32(a.wrapping_add(*b)))
        }
        (BinaryOp::SubInt32, Literal::I32(a), Literal::I32(b)) => {
            Some(Literal::I32(a.wrapping_sub(*b)))
        }
        (BinaryOp::MulInt32, Literal::I32(a), Literal::I32(b)) => {
            Some(Literal::I32(a.wrapping_mul(*b)))
        }
        (BinaryOp::DivSInt32, Literal::I32(a), Literal::I32(b)) if *b != 0 => {
            Some(Literal::I32(a.wrapping_div(*b)))
        }
        (BinaryOp::DivUInt32, Literal::I32(a), Literal::I32(b)) if *b != 0 => {
            Some(Literal::I32(((*a as u32) / (*b as u32)) as i32))
        }
        (BinaryOp::AndInt32, Literal::I32(a), Literal::I32(b)) => Some(Literal::I32(a & b)),
        (BinaryOp::OrInt32, Literal::I32(a), Literal::I32(b)) => Some(Literal::I32(a | b)),
        (BinaryOp::XorInt32, Literal::I32(a), Literal::I32(b)) => Some(Literal::I32(a ^ b)),
        (BinaryOp::EqInt32, Literal::I32(a), Literal::I32(b)) => {
            Some(Literal::I32(if a == b { 1 } else { 0 }))
        }
        _ => None,
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
    fn test_precompute_add() {
        let bump = Bump::new();
        let left = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(10)),
            type_: Type::I32,
        }));
        let right = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(20)),
            type_: Type::I32,
        }));
        let binary = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Binary {
                op: BinaryOp::AddInt32,
                left,
                right,
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
        let bump = bumpalo::Bump::new();
        let mut module = Module::new(&bump);
        module.add_function(func);
        let mut pass = Precompute;
        pass.run(&mut module);
        let body = module.functions[0].body.as_ref().unwrap();
        assert!(matches!(body.kind, ExpressionKind::Const(Literal::I32(30))));
    }
}
