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
            ExpressionKind::Call { operands, .. } => {
                for operand in operands.iter_mut() {
                    self.visit(operand);
                }
            }
            ExpressionKind::LocalSet { value, .. } | ExpressionKind::LocalTee { value, .. } => {
                self.visit(value);
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.visit(condition);
                self.visit(if_true);
                if let Some(false_branch) = if_false {
                    self.visit(false_branch);
                }
            }
            ExpressionKind::Loop { body, .. } => {
                self.visit(body);
            }
            ExpressionKind::Break {
                condition, value, ..
            } => {
                if let Some(cond) = condition {
                    self.visit(cond);
                }
                if let Some(val) = value {
                    self.visit(val);
                }
            }
            ExpressionKind::Const(_) | ExpressionKind::Nop | ExpressionKind::LocalGet { .. } => {}
        }
    }
}
