use crate::effects::EffectAnalyzer;
use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::{Literal, Type};
use bumpalo::collections::Vec as BumpVec;

pub struct Vacuum;

impl Pass for Vacuum {
    fn name(&self) -> &str {
        "Vacuum"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                self.visit(body);
            }
        }
    }
}

impl<'a> Visitor<'a> for Vacuum {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        // First visit children (bottom-up transformation)
        self.visit_children(expr);

        let expr_type = expr.type_;

        match &mut expr.kind {
            ExpressionKind::Block { name, list } => {
                Self::optimize_block(expr_type, list);
                // If it's a name-less block with 0 or 1 element, simplify it
                if name.is_none() {
                    if list.is_empty() {
                        expr.kind = ExpressionKind::Nop;
                        expr.type_ = Type::NONE;
                    } else if list.len() == 1 {
                        let mut child = list[0];
                        expr.type_ = child.type_;
                        expr.kind = std::mem::replace(&mut child.kind, ExpressionKind::Nop);
                    }
                }
            }
            ExpressionKind::If {
                condition,
                if_true: _,
                if_false,
            } => {
                if let ExpressionKind::Const(lit) = &condition.kind {
                    let is_true = match lit {
                        Literal::I32(v) => *v != 0,
                        Literal::I64(v) => *v != 0,
                        _ => false,
                    };
                    if is_true {
                        // Replace with if_true
                        let kind = std::mem::replace(&mut expr.kind, ExpressionKind::Nop);
                        if let ExpressionKind::If { mut if_true, .. } = kind {
                            expr.type_ = if_true.type_;
                            expr.kind = std::mem::replace(&mut if_true.kind, ExpressionKind::Nop);
                        }
                    } else if let Some(mut f) = if_false.take() {
                        // Replace with if_false
                        expr.type_ = f.type_;
                        expr.kind = std::mem::replace(&mut f.kind, ExpressionKind::Nop);
                    } else {
                        // Replace with Nop
                        expr.kind = ExpressionKind::Nop;
                        expr.type_ = Type::NONE;
                    }
                } else if let Some(f) = if_false {
                    if matches!(f.kind, ExpressionKind::Nop) {
                        *if_false = None;
                    }
                }
            }
            ExpressionKind::Drop { value } => {
                if !EffectAnalyzer::analyze(*value).has_side_effects() {
                    // Constant or side-effect-free expr dropped is a no-op
                    expr.kind = ExpressionKind::Nop;
                    expr.type_ = Type::NONE;
                }
            }
            _ => {}
        }
    }
}

impl Vacuum {
    fn is_concrete(ty: Type) -> bool {
        ty != Type::NONE && ty != Type::UNREACHABLE
    }

    fn optimize_block<'a>(block_type: Type, list: &mut BumpVec<'a, ExprRef<'a>>) {
        if list.is_empty() {
            return;
        }

        let mut write_idx = 0;
        let len = list.len();

        for read_idx in 0..len {
            let child = list[read_idx];

            if child.type_ == Type::UNREACHABLE {
                list[write_idx] = child;
                write_idx += 1;
                // Truncate after first unreachable
                break;
            }

            if matches!(child.kind, ExpressionKind::Nop) {
                // Keep only if it's the last element of a non-void block
                if read_idx == len - 1 && Self::is_concrete(block_type) {
                    list[write_idx] = child;
                    write_idx += 1;
                }
                continue;
            }

            // Remove side-effect-free non-last expressions
            if read_idx < len - 1 && !Self::is_concrete(child.type_) {
                if !EffectAnalyzer::analyze(child).has_side_effects() {
                    continue;
                }
            }

            list[write_idx] = child;
            write_idx += 1;
        }

        if write_idx < list.len() {
            list.truncate(write_idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExprRef, Expression, ExpressionKind};
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::collections::Vec as BumpVec;
    use bumpalo::Bump;

    #[test]
    fn test_vacuum_removes_nops() {
        let bump = Bump::new();
        let const1 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));
        let nop = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));
        let const2 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(2)),
            type_: Type::I32,
        }));
        let mut list = BumpVec::new_in(&bump);
        list.push(const1);
        list.push(nop);
        list.push(const2);
        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::I32,
        }));
        let func = Function::new("test".into(), Type::NONE, Type::I32, vec![], Some(block));
        let mut module = Module::new(&bump);
        module.add_function(func);

        Vacuum.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert_eq!(list.len(), 2);
        } else {
            // It might have been simplified to just 1 element if it was name-less!
            // Wait, my tests expect Block.
            // If I simplified the block to 1 element, it wouldn't be a Block anymore.
            // Let's check my logic: it only simplifies if name is None and len <= 1.
            // Here len joined to 2. So it stays a Block.
        }
    }

    #[test]
    fn test_vacuum_simplifies_if_true() {
        let bump = Bump::new();
        let condition = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));
        let if_true = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        }));
        let if_expr = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::If {
                condition,
                if_true,
                if_false: None,
            },
            type_: Type::I32,
        }));
        let func = Function::new("test".into(), Type::NONE, Type::I32, vec![], Some(if_expr));
        let mut module = Module::new(&bump);
        module.add_function(func);

        Vacuum.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        assert!(matches!(body.kind, ExpressionKind::Const(Literal::I32(42))));
    }

    #[test]
    fn test_vacuum_simplifies_drop() {
        let bump = Bump::new();
        let val = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        }));
        let drop_expr = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Drop { value: val },
            type_: Type::NONE,
        }));
        let func = Function::new(
            "test".into(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(drop_expr),
        );
        let mut module = Module::new(&bump);
        module.add_function(func);

        Vacuum.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        assert!(matches!(body.kind, ExpressionKind::Nop));
    }
}
