pub mod expression;
pub mod ops;
pub mod visitor;

pub use expression::{ExprRef, Expression, ExpressionKind, IrBuilder};
pub use ops::{BinaryOp, UnaryOp};
pub use visitor::Visitor;

#[cfg(test)]
mod tests {
    use super::*;
    use binaryen_core::{Literal, Type};
    use bumpalo::collections::Vec as BumpVec;
    use bumpalo::Bump;

    #[test]
    fn test_ir_construction() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let const_expr = builder.const_(Literal::I32(42));

        match const_expr.kind {
            ExpressionKind::Const(Literal::I32(42)) => (),
            _ => panic!("Expected Const(42)"),
        }
        assert_eq!(const_expr.type_, Type::I32);

        let mut list = BumpVec::new_in(&bump);
        list.push(const_expr);

        let block = builder.block(Some("my_block"), list, Type::I32);

        if let ExpressionKind::Block { name, list } = &block.kind {
            assert_eq!(*name, Some("my_block"));
            assert_eq!(list.len(), 1);
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_binary_op() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let left = builder.const_(Literal::I32(10));
        let right = builder.const_(Literal::I32(32));

        let add = builder.binary(BinaryOp::AddInt32, left, right, Type::I32);

        if let ExpressionKind::Binary { op, left, right } = &add.kind {
            assert_eq!(*op, BinaryOp::AddInt32);
            assert_eq!(left.type_, Type::I32);
            assert_eq!(right.type_, Type::I32);
        } else {
            panic!("Expected Binary");
        }
    }

    struct CountVisitor {
        count: usize,
    }

    impl<'a> Visitor<'a> for CountVisitor {
        fn visit_expression(&mut self, _expr: &mut Expression<'a>) {
            self.count += 1;
        }
    }

    #[test]
    fn test_visitor() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let c1 = builder.const_(Literal::I32(1));
        let c2 = builder.const_(Literal::I32(2));
        let add = builder.binary(BinaryOp::AddInt32, c1, c2, Type::I32);

        let mut list = BumpVec::new_in(&bump);
        list.push(add);
        let block = builder.block(None, list, Type::I32);

        let mut v = CountVisitor { count: 0 };
        v.visit(block);

        // Block (1) -> Add (1) -> Const (1) + Const (1) = 4 expressions
        assert_eq!(v.count, 4);
    }
}
