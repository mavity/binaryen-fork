use crate::expression::{ExprRef, ExpressionKind};

pub trait Visitor<'a> {
    fn visit(&mut self, expr: &mut ExprRef<'a>) {
        self.visit_expression(expr);
        self.visit_children(expr);
    }

    fn visit_expression(&mut self, _expr: &mut ExprRef<'a>) {}

    fn visit_children(&mut self, expr: &mut ExprRef<'a>) {
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
            ExpressionKind::GlobalSet { value, .. } => {
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
            ExpressionKind::Return { value } => {
                if let Some(val) = value {
                    self.visit(val);
                }
            }
            ExpressionKind::Drop { value } => {
                self.visit(value);
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                self.visit(condition);
                self.visit(if_true);
                self.visit(if_false);
            }
            ExpressionKind::Load { ptr, .. } => {
                self.visit(ptr);
            }
            ExpressionKind::Store { ptr, value, .. } => {
                self.visit(ptr);
                self.visit(value);
            }
            ExpressionKind::Switch {
                condition, value, ..
            } => {
                self.visit(condition);
                if let Some(val) = value {
                    self.visit(val);
                }
            }
            ExpressionKind::CallIndirect {
                target, operands, ..
            } => {
                self.visit(target);
                for operand in operands.iter_mut() {
                    self.visit(operand);
                }
            }
            ExpressionKind::MemoryGrow { delta } => {
                self.visit(delta);
            }
            ExpressionKind::AtomicRMW { ptr, value, .. } => {
                self.visit(ptr);
                self.visit(value);
            }
            ExpressionKind::AtomicCmpxchg {
                ptr,
                expected,
                replacement,
                ..
            } => {
                self.visit(ptr);
                self.visit(expected);
                self.visit(replacement);
            }
            ExpressionKind::AtomicWait {
                ptr,
                expected,
                timeout,
                ..
            } => {
                self.visit(ptr);
                self.visit(expected);
                self.visit(timeout);
            }
            ExpressionKind::AtomicNotify { ptr, count, .. } => {
                self.visit(ptr);
                self.visit(count);
            }
            ExpressionKind::SIMDExtract { vec, .. } => {
                self.visit(vec);
            }
            ExpressionKind::SIMDReplace { vec, value, .. } => {
                self.visit(vec);
                self.visit(value);
            }
            ExpressionKind::SIMDShuffle { left, right, .. } => {
                self.visit(left);
                self.visit(right);
            }
            ExpressionKind::SIMDTernary { a, b, c, .. } => {
                self.visit(a);
                self.visit(b);
                self.visit(c);
            }
            ExpressionKind::SIMDShift { vec, shift, .. } => {
                self.visit(vec);
                self.visit(shift);
            }
            ExpressionKind::SIMDLoad { ptr, .. } => {
                self.visit(ptr);
            }
            ExpressionKind::SIMDLoadStoreLane { ptr, vec, .. } => {
                self.visit(ptr);
                self.visit(vec);
            }
            ExpressionKind::MemoryInit {
                dest, offset, size, ..
            } => {
                self.visit(dest);
                self.visit(offset);
                self.visit(size);
            }
            ExpressionKind::MemoryCopy {
                dest, src, size, ..
            } => {
                self.visit(dest);
                self.visit(src);
                self.visit(size);
            }
            ExpressionKind::MemoryFill {
                dest, value, size, ..
            } => {
                self.visit(dest);
                self.visit(value);
                self.visit(size);
            }
            ExpressionKind::Unreachable
            | ExpressionKind::Const(_)
            | ExpressionKind::Nop
            | ExpressionKind::LocalGet { .. }
            | ExpressionKind::GlobalGet { .. }
            | ExpressionKind::MemorySize
            | ExpressionKind::AtomicFence
            | ExpressionKind::DataDrop { .. } => {}
        }
    }
}

pub trait ReadOnlyVisitor<'a> {
    fn visit(&mut self, expr: ExprRef<'a>) {
        self.visit_expression(expr);
        self.visit_children(expr);
    }

    fn visit_expression(&mut self, _expr: ExprRef<'a>) {}

    fn visit_children(&mut self, expr: ExprRef<'a>) {
        match &expr.kind {
            ExpressionKind::Block { list, .. } => {
                for child in list.iter() {
                    self.visit(child.clone());
                }
            }
            ExpressionKind::Unary { value, .. } => {
                self.visit(value.clone());
            }
            ExpressionKind::Binary { left, right, .. } => {
                self.visit(left.clone());
                self.visit(right.clone());
            }
            ExpressionKind::Call { operands, .. } => {
                for operand in operands.iter() {
                    self.visit(operand.clone());
                }
            }
            ExpressionKind::LocalSet { value, .. } | ExpressionKind::LocalTee { value, .. } => {
                self.visit(value.clone());
            }
            ExpressionKind::GlobalSet { value, .. } => {
                self.visit(value.clone());
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.visit(condition.clone());
                self.visit(if_true.clone());
                if let Some(false_branch) = if_false {
                    self.visit(false_branch.clone());
                }
            }
            ExpressionKind::Loop { body, .. } => {
                self.visit(body.clone());
            }
            ExpressionKind::Break {
                condition, value, ..
            } => {
                if let Some(cond) = condition {
                    self.visit(cond.clone());
                }
                if let Some(val) = value {
                    self.visit(val.clone());
                }
            }
            ExpressionKind::Return { value } => {
                if let Some(val) = value {
                    self.visit(val.clone());
                }
            }
            ExpressionKind::Drop { value } => {
                self.visit(value.clone());
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                self.visit(condition.clone());
                self.visit(if_true.clone());
                self.visit(if_false.clone());
            }
            ExpressionKind::Load { ptr, .. } => {
                self.visit(ptr.clone());
            }
            ExpressionKind::Store { ptr, value, .. } => {
                self.visit(ptr.clone());
                self.visit(value.clone());
            }
            ExpressionKind::Switch {
                condition, value, ..
            } => {
                self.visit(condition.clone());
                if let Some(val) = value {
                    self.visit(val.clone());
                }
            }
            ExpressionKind::CallIndirect {
                target, operands, ..
            } => {
                self.visit(target.clone());
                for operand in operands.iter() {
                    self.visit(operand.clone());
                }
            }
            ExpressionKind::MemoryGrow { delta } => {
                self.visit(delta.clone());
            }
            ExpressionKind::AtomicRMW { ptr, value, .. } => {
                self.visit(ptr.clone());
                self.visit(value.clone());
            }
            ExpressionKind::AtomicCmpxchg {
                ptr,
                expected,
                replacement,
                ..
            } => {
                self.visit(ptr.clone());
                self.visit(expected.clone());
                self.visit(replacement.clone());
            }
            ExpressionKind::AtomicWait {
                ptr,
                expected,
                timeout,
                ..
            } => {
                self.visit(ptr.clone());
                self.visit(expected.clone());
                self.visit(timeout.clone());
            }
            ExpressionKind::AtomicNotify { ptr, count, .. } => {
                self.visit(ptr.clone());
                self.visit(count.clone());
            }
            ExpressionKind::SIMDExtract { vec, .. } => {
                self.visit(vec.clone());
            }
            ExpressionKind::SIMDReplace { vec, value, .. } => {
                self.visit(vec.clone());
                self.visit(value.clone());
            }
            ExpressionKind::SIMDShuffle { left, right, .. } => {
                self.visit(left.clone());
                self.visit(right.clone());
            }
            ExpressionKind::SIMDTernary { a, b, c, .. } => {
                self.visit(a.clone());
                self.visit(b.clone());
                self.visit(c.clone());
            }
            ExpressionKind::SIMDShift { vec, shift, .. } => {
                self.visit(vec.clone());
                self.visit(shift.clone());
            }
            ExpressionKind::SIMDLoad { ptr, .. } => {
                self.visit(ptr.clone());
            }
            ExpressionKind::SIMDLoadStoreLane { ptr, vec, .. } => {
                self.visit(ptr.clone());
                self.visit(vec.clone());
            }
            ExpressionKind::MemoryInit {
                dest, offset, size, ..
            } => {
                self.visit(dest.clone());
                self.visit(offset.clone());
                self.visit(size.clone());
            }
            ExpressionKind::MemoryCopy {
                dest, src, size, ..
            } => {
                self.visit(dest.clone());
                self.visit(src.clone());
                self.visit(size.clone());
            }
            ExpressionKind::MemoryFill {
                dest, value, size, ..
            } => {
                self.visit(dest.clone());
                self.visit(value.clone());
                self.visit(size.clone());
            }
            ExpressionKind::Unreachable
            | ExpressionKind::Const(_)
            | ExpressionKind::Nop
            | ExpressionKind::LocalGet { .. }
            | ExpressionKind::GlobalGet { .. }
            | ExpressionKind::MemorySize
            | ExpressionKind::AtomicFence
            | ExpressionKind::DataDrop { .. } => {}
        }
    }
}
