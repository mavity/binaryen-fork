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
        // First visit children
        self.visit_children(expr);

        // Then optimize the current expression
        // Copy type before borrowing expr.kind mutably
        let expr_type = expr.type_;

        match &mut expr.kind {
            ExpressionKind::Block { list, .. } => {
                self.optimize_block(expr_type, list);
            }
            ExpressionKind::If {
                condition,
                if_true: _,
                if_false,
            } => {
                self.optimize_if(condition, if_false);
            }
            ExpressionKind::Drop { value } => {
                self.optimize_drop(value);
            }
            _ => {}
        }
    }
}

impl Vacuum {
    fn is_concrete(ty: Type) -> bool {
        ty != Type::NONE && ty != Type::UNREACHABLE
    }

    fn optimize_block<'a>(&self, block_type: Type, list: &mut BumpVec<'a, ExprRef<'a>>) {
        if list.is_empty() {
            return;
        }

        // Remove nops and dead code
        let mut write_idx = 0;
        let len = list.len();

        for read_idx in 0..len {
            let child = list[read_idx];

            // Keep unreachable expressions - they mark control flow boundaries
            if child.type_ == Type::UNREACHABLE {
                list[write_idx] = child;
                write_idx += 1;
                // Everything after unreachable is dead - truncate
                if read_idx < len - 1 {
                    break;
                }
                continue;
            }

            // Check if this is a nop
            if matches!(child.kind, ExpressionKind::Nop) {
                // Skip nops unless it's the last element and we need a value
                if read_idx == len - 1 && Self::is_concrete(block_type) {
                    // Need to keep it or replace with zero
                    list[write_idx] = child;
                    write_idx += 1;
                }
                continue;
            }

            // For non-concrete expressions that have no side effects, we can remove them
            // unless they're the last element
            let is_last = read_idx == len - 1;
            if !is_last && !Self::is_concrete(child.type_) {
                let effects = EffectAnalyzer::analyze(child);
                if !effects.has_side_effects() {
                    // Can safely skip this
                    continue;
                }
            }

            // Keep this element
            list[write_idx] = child;
            write_idx += 1;
        }

        // Truncate to the number of kept elements
        if write_idx < len {
            list.truncate(write_idx);
        }
    }

    fn optimize_if<'a>(&self, condition: &mut ExprRef<'a>, if_false: &mut Option<ExprRef<'a>>) {
        // If condition is a constant, we can optimize
        if let ExpressionKind::Const(lit) = &condition.kind {
            // Check if condition is truthy
            let is_true = match lit {
                Literal::I32(v) => *v != 0,
                Literal::I64(v) => *v != 0,
                _ => return, // Can't optimize non-integer constants in condition
            };

            // Replace the entire if with the appropriate branch
            // Note: This requires arena-based manipulation which is not yet implemented
            // For now, we just detect the optimization opportunity
            let _ = is_true; // Suppress unused warning
        }

        // If condition is unreachable, the whole if is unreachable
        // This would require replacing the entire if with just the condition

        // Remove nop branches
        if let Some(false_branch) = if_false {
            if matches!(false_branch.kind, ExpressionKind::Nop) {
                *if_false = None;
            }
        }

        // If true branch is nop and we have a false branch, we can negate the condition
        // and swap branches (but this requires creating a new unary expression)
    }

    fn optimize_drop<'a>(&self, child: &mut ExprRef<'a>) {
        // If the child has no side effects, we can potentially remove the drop entirely
        let effects = EffectAnalyzer::analyze(*child);

        if !effects.has_side_effects() {
            // The dropped value has no side effects, so the drop itself does nothing
            // Would replace drop with nop - requires arena manipulation
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

        // Create a block with nops
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

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            // Nop should be removed, leaving 2 const expressions
            assert_eq!(list.len(), 2, "Expected 2 instructions after Vacuum");
            assert!(matches!(list[0].kind, ExpressionKind::Const(_)));
            assert!(matches!(list[1].kind, ExpressionKind::Const(_)));
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_removes_dead_code_after_unreachable() {
        let bump = Bump::new();

        let const1 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let unreachable = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Unreachable,
            type_: Type::UNREACHABLE,
        }));

        let dead_code = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(2)),
            type_: Type::I32,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(const1);
        list.push(unreachable);
        list.push(dead_code);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::UNREACHABLE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            // Dead code after unreachable should be removed
            assert_eq!(list.len(), 2, "Expected 2 instructions after Vacuum");
            assert!(matches!(list[0].kind, ExpressionKind::Const(_)));
            assert!(matches!(list[1].kind, ExpressionKind::Unreachable));
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_empty_block() {
        let bump = Bump::new();

        let list = BumpVec::new_in(&bump);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::NONE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        // Should not crash on empty blocks
        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert_eq!(list.len(), 0);
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_multiple_nops() {
        let bump = Bump::new();

        // Create a block with multiple consecutive nops
        let const1 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let nop1 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let nop2 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let nop3 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let const2 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(2)),
            type_: Type::I32,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(const1);
        list.push(nop1);
        list.push(nop2);
        list.push(nop3);
        list.push(const2);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::I32,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            // All nops should be removed
            assert_eq!(list.len(), 2, "Expected 2 instructions after removing nops");
            assert!(matches!(
                list[0].kind,
                ExpressionKind::Const(Literal::I32(1))
            ));
            assert!(matches!(
                list[1].kind,
                ExpressionKind::Const(Literal::I32(2))
            ));
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_multiple_dead_after_unreachable() {
        let bump = Bump::new();

        let const1 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let unreachable = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Unreachable,
            type_: Type::UNREACHABLE,
        }));

        // Multiple dead instructions
        let dead1 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(2)),
            type_: Type::I32,
        }));

        let dead2 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(3)),
            type_: Type::I32,
        }));

        let dead3 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(const1);
        list.push(unreachable);
        list.push(dead1);
        list.push(dead2);
        list.push(dead3);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::UNREACHABLE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            // All code after unreachable should be removed
            assert_eq!(
                list.len(),
                2,
                "Expected only 2 instructions after unreachable"
            );
            assert!(matches!(list[0].kind, ExpressionKind::Const(_)));
            assert!(matches!(list[1].kind, ExpressionKind::Unreachable));
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_nested_blocks() {
        let bump = Bump::new();

        // Inner block with nops
        let inner_const = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(5)),
            type_: Type::I32,
        }));

        let inner_nop = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let mut inner_list = BumpVec::new_in(&bump);
        inner_list.push(inner_nop);
        inner_list.push(inner_const);

        let inner_block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: None,
                list: inner_list,
            },
            type_: Type::I32,
        }));

        // Outer block
        let outer_const = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(10)),
            type_: Type::I32,
        }));

        let outer_nop = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let mut outer_list = BumpVec::new_in(&bump);
        outer_list.push(inner_block);
        outer_list.push(outer_nop);
        outer_list.push(outer_const);

        let outer_block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: None,
                list: outer_list,
            },
            type_: Type::I32,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(outer_block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            // Outer nop should be removed
            assert_eq!(list.len(), 2, "Expected 2 instructions in outer block");

            // Check inner block was also cleaned
            if let ExpressionKind::Block {
                list: inner_list, ..
            } = &list[0].kind
            {
                assert_eq!(inner_list.len(), 1, "Expected 1 instruction in inner block");
                assert!(matches!(
                    inner_list[0].kind,
                    ExpressionKind::Const(Literal::I32(5))
                ));
            } else {
                panic!("Expected inner Block");
            }

            assert!(matches!(
                list[1].kind,
                ExpressionKind::Const(Literal::I32(10))
            ));
        } else {
            panic!("Expected outer Block");
        }
    }

    #[test]
    fn test_vacuum_nop_removal_in_if_branches() {
        let bump = Bump::new();

        let condition = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let if_true = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        }));

        let if_false_nop = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let if_expr = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::If {
                condition,
                if_true,
                if_false: Some(if_false_nop),
            },
            type_: Type::I32,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(if_expr),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::If { if_false, .. } = &body.kind {
            // Nop in else branch should be removed
            assert!(if_false.is_none(), "Expected else branch to be removed");
        } else {
            panic!("Expected If");
        }
    }

    #[test]
    fn test_vacuum_preserves_side_effects() {
        let bump = Bump::new();

        // LocalSet has side effects and should not be removed
        let value = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        }));

        let local_set = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::LocalSet { index: 0, value },
            type_: Type::NONE,
        }));

        let const_val = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(local_set);
        list.push(const_val);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::I32,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![Type::I32],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            // LocalSet should be preserved due to side effects
            assert_eq!(list.len(), 2, "Expected local.set to be preserved");
            assert!(matches!(list[0].kind, ExpressionKind::LocalSet { .. }));
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_only_nops() {
        let bump = Bump::new();

        let nop1 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let nop2 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(nop1);
        list.push(nop2);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::NONE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            // All nops should be removed for block with NONE type
            assert_eq!(list.len(), 0, "Expected all nops to be removed");
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_unreachable_at_start() {
        let bump = Bump::new();

        let unreachable = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Unreachable,
            type_: Type::UNREACHABLE,
        }));

        let dead = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(unreachable);
        list.push(dead);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::UNREACHABLE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert_eq!(list.len(), 1, "Expected only unreachable");
            assert!(matches!(list[0].kind, ExpressionKind::Unreachable));
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_vacuum_mixed_types() {
        let bump = Bump::new();

        // Mix of i32, i64, f32 constants with nops
        let i32_const = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let nop = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let i64_const = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I64(2)),
            type_: Type::I64,
        }));

        let nop2 = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }));

        let f32_const = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::F32(3.0)),
            type_: Type::F32,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(i32_const);
        list.push(nop);
        list.push(i64_const);
        list.push(nop2);
        list.push(f32_const);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name: None, list },
            type_: Type::F32,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::F32,
            vec![],
            Some(block),
        );

        let mut module = Module::new();
        module.add_function(func);

        let mut pass = Vacuum;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert_eq!(list.len(), 3, "Expected 3 const expressions");
            assert!(matches!(
                list[0].kind,
                ExpressionKind::Const(Literal::I32(1))
            ));
            assert!(matches!(
                list[1].kind,
                ExpressionKind::Const(Literal::I64(2))
            ));
            assert!(matches!(
                list[2].kind,
                ExpressionKind::Const(Literal::F32(_))
            ));
        } else {
            panic!("Expected Block");
        }
    }
}
