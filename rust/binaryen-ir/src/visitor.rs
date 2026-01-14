use crate::expression::{Expression, ExpressionKind};

pub trait Visitor<'a> {
    fn visit(&mut self, expr: &mut Expression<'a>) {
        self.visit_expression(expr);
        self.visit_children(expr);
    }

    fn visit_expression(&mut self, _expr: &mut Expression<'a>) {}

    fn visit_children(&mut self, expr: &mut Expression<'a>) {
        match &mut expr.kind {
            ExpressionKind::Block { list, .. } => {
                for child in list.iter_mut() {
                    self.visit(child);
                }
            }
            ExpressionKind::Unary { value, .. } => {
                self.visit(value);
            }
            ExpressionKind::Binary { left, right, .. } => {
                self.visit(left);
                self.visit(right);
            }
            ExpressionKind::Const(_) | ExpressionKind::Nop => {}
        }
    }
}
