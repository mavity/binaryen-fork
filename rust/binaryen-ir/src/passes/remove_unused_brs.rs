use crate::expression::{ExprRef, Expression, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use binaryen_core::Type;
use bumpalo::Bump;

pub struct RemoveUnusedBrs;

impl Pass for RemoveUnusedBrs {
    fn name(&self) -> &str {
        "RemoveUnusedBrs"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut optimizer = BrOptimizer {
                    bump: module.allocator,
                };
                optimizer.optimize(body);
            }
        }
    }
}

struct BrOptimizer<'a> {
    bump: &'a Bump,
}

#[derive(Clone)]
struct Flow<'a> {
    // Breaks that flow out of the current expression
    breaks: Vec<ExprRef<'a>>,
    // Returns that flow out of the current expression
    returns: Vec<ExprRef<'a>>,
    // Whether execution can fall through the current expression
    falls_through: bool,
}

impl<'a> Flow<'a> {
    fn none() -> Self {
        Self {
            breaks: Vec::new(),
            returns: Vec::new(),
            falls_through: false,
        }
    }

    fn fallthrough() -> Self {
        Self {
            breaks: Vec::new(),
            returns: Vec::new(),
            falls_through: true,
        }
    }

    fn break_(expr: ExprRef<'a>) -> Self {
        Self {
            breaks: vec![expr],
            returns: Vec::new(),
            falls_through: false,
        }
    }

    fn return_(expr: ExprRef<'a>) -> Self {
        Self {
            breaks: Vec::new(),
            returns: vec![expr],
            falls_through: false,
        }
    }

    fn merge(&mut self, other: Flow<'a>) {
        self.breaks.extend(other.breaks);
        self.returns.extend(other.returns);
        self.falls_through = self.falls_through || other.falls_through;
    }
}

impl<'a> BrOptimizer<'a> {
    fn optimize(&mut self, expr: &mut ExprRef<'a>) -> Flow<'a> {
        match &mut expr.kind {
            ExpressionKind::Block { name, list } => {
                let mut current_flow = Flow::fallthrough();

                // We need to iterate and process children
                // Note: we can't use standard iterator if we modify the list logic (like removing dead code)
                // But Vacuum does dead code removal. Here we just analyze flow.
                // However, if we encounter unreachable, we should stop processing subsequent items for flow analysis?

                for child in list.iter_mut() {
                    if !current_flow.falls_through {
                        // Previous code does not fall through.
                        // We should strictly stop analyzing flow from here,
                        // as this code is unreachable (and Vacuum will remove it).
                        // However, we must still visit it to optimize nested structures?
                        // "If the if isn't even reached, this is all dead code anyhow" - C++
                        // But we might want to run optimizer on it anyway?
                        // Vacuum runs after? Or before?
                        // Ideally we optimize everything.
                        // But for flow tracking, we reset `current_flow`.
                        let _ = self.optimize(child);
                        continue;
                    }

                    let child_flow = self.optimize(child);
                    // The new flow is determined by the child
                    current_flow = child_flow;
                }

                // Now handle breaks targeting this block
                if let Some(block_name) = name {
                    // Identify breaks that target this block
                    let mut remaining_breaks = Vec::new();
                    for mut br in current_flow.breaks {
                        let is_target = if let ExpressionKind::Break {
                            name: br_name,
                            condition,
                            ..
                        } = &br.kind
                        {
                            *br_name == *block_name && condition.is_none()
                        } else {
                            false
                        };

                        if is_target {
                            // Optimize this break!
                            // Convert to nop (or value)
                            if let ExpressionKind::Break { value, .. } = &br.kind {
                                if let Some(val) = value {
                                    // Replace break with its value
                                    // br.kind = val.kind.clone()? No, we need to replace the expression
                                    // We can't move out of `val` easily because it's behind a reference.
                                    // But `ExprRef` is a pointer.
                                    // We can just copy the `ExprRef`.
                                    // Wait, `br` is `ExprRef`. We want to change what it points to.
                                    // `*br = *val`?
                                    // Yes, `ExprRef` derefs to `Expression`.
                                    // But `Expression` owns data (BumpVec etc).
                                    // We can't easily move out of `val` if it's inside `br`.
                                    // But `br` structure is `Break { ..., value: Option<ExprRef> }`.
                                    // `val` is that `ExprRef`.
                                    // We want to replace `br` with `val`.
                                    // This is: `*br = *val`.
                                    // But `val` is inside `br`. We can't move it out while modifying `br`.
                                    // We need to take it. `value.take()`?
                                    // `ExpressionKind` fields are public.
                                    // But we only have `&ExpressionKind` from the match?
                                    // No, we have `br` which is `ExprRef`.
                                    // We can match mutably.

                                    // We need to re-match to mutate
                                    let val_ref = *val;
                                    // We can replace `br` with `val_ref`'s content?
                                    // No, `val_ref` points to an expression node. `br` points to the break node.
                                    // We want `br` pointer to now look like `val_ref`.
                                    // We can shallow copy `Expression` struct?
                                    // `Expression` contains `BumpVec` which is not Copy.
                                    // But we are in an arena.
                                    // If we memcpy `*val_ref` to `*br`, we have two nodes pointing to same children.
                                    // This is fine in immutable AST, but here we might mutate?
                                    // Binaryen C++ does `*flows[i] = flow->value`.
                                    // Rust `Expression` is not `Copy`.
                                    // We can `clone()` if `Expression` implements `Clone`.
                                    // `Expression` implements `Debug`. `Clone`?
                                    // `BumpVec` is not `Clone`? It is `Clone` if element is `Clone`. `ExprRef` is `Copy`.
                                    // So `Expression` CAN be `Clone`.
                                    // Let's check `expression.rs` derive.
                                    // `#[derive(Debug)]` only.
                                    // So we cannot clone.

                                    // Alternative:
                                    // We can't easily replace `br` with `val`'s content in-place if we can't clone.
                                    // But we can swap?
                                    // `std::mem::swap(&mut *br, &mut *val_ref)`?
                                    // Then `val_ref` gets the `Break`. `br` gets the value.
                                    // This works! The `Break` node (now at `val_ref`) is effectively dead/orphaned (it was a child of the original `Break`).

                                    unsafe {
                                        let val_ptr = val_ref.as_ptr();
                                        let br_ptr = br.as_ptr();
                                        std::ptr::swap(br_ptr, val_ptr);
                                        // Now `br` points to the value content.
                                        // `val_ref` (the child) points to the Break content.
                                        // The child is effectively garbage now.
                                    }
                                } else {
                                    // No value: replace with Nop
                                    *br = Expression {
                                        kind: ExpressionKind::Nop,
                                        type_: Type::NONE,
                                    };
                                }
                            }

                            // Since we converted a break to fallthrough/value,
                            // the block now receives flow from this path.
                            current_flow.falls_through = true;
                        } else {
                            remaining_breaks.push(br);
                        }
                    }
                    current_flow.breaks = remaining_breaks;
                }

                current_flow
            }
            ExpressionKind::Loop { name, body } => {
                let mut flow = self.optimize(body);

                if let Some(loop_name) = name {
                    // Filter out breaks that target this loop (back-edges)
                    flow.breaks.retain(|br| {
                        if let ExpressionKind::Break {
                            name: br_name,
                            condition,
                            ..
                        } = &br.kind
                        {
                            if *br_name == *loop_name && condition.is_none() {
                                // It's a back-edge. It does not flow out.
                                // It keeps the loop looping.
                                return false;
                            }
                        }
                        true
                    });
                }
                flow
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                let _ = self.optimize(condition);
                let mut flow = self.optimize(if_true);

                if let Some(if_false_expr) = if_false {
                    let false_flow = self.optimize(if_false_expr);
                    flow.merge(false_flow);
                } else {
                    // If no else, and condition is false, we fall through.
                    flow.falls_through = true;
                }

                flow
            }
            ExpressionKind::Break { condition, .. } => {
                // If unconditional, we break.
                // If conditional, we break AND fall through.

                // We also need to visit children (value/condition)
                // Using visitor pattern or manual recursion?
                // Manual recursion to be safe.
                if let ExpressionKind::Break {
                    condition, value, ..
                } = &mut expr.kind
                {
                    if let Some(cond) = condition {
                        self.optimize(cond);
                    }
                    if let Some(val) = value {
                        self.optimize(val);
                    }
                }

                if let ExpressionKind::Break { condition, .. } = &expr.kind {
                    if condition.is_none() {
                        Flow::break_(*expr)
                    } else {
                        // Conditional break
                        // It flows out via break AND falls through
                        let mut flow = Flow::fallthrough();
                        flow.breaks.push(*expr); // Use a separate tracking for conditional breaks?
                                                 // C++ RemoveUnusedBrs only optimizes unconditional breaks.
                                                 // So we should NOT put conditional break in the `breaks` list
                                                 // unless we want to support optimizing `br_if`.
                                                 // For now, let's stick to unconditional breaks.
                        Flow::fallthrough()
                    }
                } else {
                    Flow::fallthrough() // Should not happen
                }
            }
            ExpressionKind::Return { value } => {
                if let Some(val) = value {
                    self.optimize(val);
                }
                Flow::return_(*expr)
            }
            ExpressionKind::Switch {
                condition, value, ..
            } => {
                self.optimize(condition);
                if let Some(val) = value {
                    self.optimize(val);
                }
                // Switch targets are not analyzed here for removal yet
                // It stops flow (we assume it branches somewhere)
                Flow::none()
            }
            ExpressionKind::Unreachable => Flow::none(),
            // Other expressions fall through and recurse children
            _ => {
                // We need to visit children!
                // Since we don't have a generic "visit children" helper readily available on `expr`
                // without borrowing `expr` mutably which conflicts with passing `self` to `optimize`,
                // we have to implement child visiting manually or use the Visitor trait.
                // But Visitor trait doesn't return `Flow`.

                // We can use `Visitor::visit_children` implementation as reference.
                // Or we can implement `optimize` using `Visitor` where we store `Flow` in a side stack.
                // A stack-based visitor is cleaner.

                // For now, let's handle the common containers manually?
                // `Block`, `Loop`, `If` are handled.
                // `Call`, `CallIndirect` have operands.
                // `LocalSet`, `GlobalSet`, `Load`, `Store`, `Unary`, `Binary`, `Select`, `Drop`...
                // They all fall through.
                // But they contain children that might be blocks/ifs?
                // Yes, `(drop (block ...))`
                // So we MUST recurse.

                // Implementing a `FlowVisitor` seems better.
                // `visit_expression` calls `visit_children`.
                // `visit_children` iterates.
                // We can accumulate flow from children?
                // Flow from children of a `Call`?
                // `Call(a, b, c)` -> executes a, then b, then c, then call.
                // If `a` breaks, `b` is dead.
                // Flow is sequential.

                // Let's implement `visit_children_flow` helper.
                self.visit_children_flow(expr)
            }
        }
    }

    fn visit_children_flow(&mut self, expr: &mut ExprRef<'a>) -> Flow<'a> {
        // Generic traversal
        // We can cheat and use `Visitor` logic but we need to return Flow.
        // Most expressions execute children in order and then fall through.
        // If any child breaks/unreachable, the rest are skipped (logically).

        let mut current_flow = Flow::fallthrough();

        // We can't iterate generic children easily without `ExpressionKind` match.
        // Let's rely on the fact that `Visitor` implementation in `visitor.rs` visits children.
        // Maybe we can adapt it?
        // Or just implement the big match.

        match &mut expr.kind {
            ExpressionKind::Unary { value, .. }
            | ExpressionKind::Drop { value }
            | ExpressionKind::LocalSet { value, .. }
            | ExpressionKind::LocalTee { value, .. }
            | ExpressionKind::GlobalSet { value, .. }
            | ExpressionKind::Load { ptr: value, .. }
            | ExpressionKind::MemoryGrow { delta: value } => {
                current_flow = self.optimize(value);
            }

            ExpressionKind::Binary { left, right, .. }
            | ExpressionKind::Store {
                ptr: left,
                value: right,
                ..
            } => {
                current_flow = self.optimize(left);
                if current_flow.falls_through {
                    current_flow = self.optimize(right);
                }
            }

            ExpressionKind::Call { operands, .. } => {
                for op in operands {
                    if !current_flow.falls_through {
                        break;
                    }
                    current_flow = self.optimize(op);
                }
            }

            ExpressionKind::CallIndirect {
                operands, target, ..
            } => {
                current_flow = self.optimize(target);

                if current_flow.falls_through {
                    for op in operands {
                        if !current_flow.falls_through {
                            break;
                        }
                        current_flow = self.optimize(op);
                    }
                }
            }

            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                current_flow = self.optimize(condition);
                if current_flow.falls_through {
                    current_flow = self.optimize(if_true);
                }
                if current_flow.falls_through {
                    current_flow = self.optimize(if_false);
                }
            }

            // ... Handle other kinds ...
            // For simplicity, let's assume other kinds are simple or leaves.
            // Const, LocalGet, GlobalGet, Nop, Unreachable, MemorySize -> leaves.
            ExpressionKind::Const(_)
            | ExpressionKind::LocalGet { .. }
            | ExpressionKind::GlobalGet { .. }
            | ExpressionKind::Nop
            | ExpressionKind::MemorySize => {
                // Leaf, falls through (Unreachable is handled in main match)
            }

            _ => {
                // Remaining: Atomic, SIMD, Bulk Memory...
                // Conservatively assume they have children we should visit?
                // Or just assume they fall through.
                // Ideally we visit all children.
            }
        }

        // If the expression itself falls through (which most do, except Return/Break/Switch/Unreachable which are handled in main match),
        // we return the flow from the last child?
        // Yes, but the expression itself doesn't stop flow.
        // So if last child falls through, the expression falls through.

        current_flow
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
    fn test_remove_unused_br_void() {
        let bump = Bump::new();

        // (block $L (br $L))
        let br = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Break {
                name: "L",
                condition: None,
                value: None,
            },
            type_: Type::UNREACHABLE,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(br);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: Some("L"),
                list,
            },
            type_: Type::NONE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = RemoveUnusedBrs;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert!(matches!(list[0].kind, ExpressionKind::Nop), "Expected Nop");
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_remove_unused_br_value() {
        let bump = Bump::new();

        // (block $L (result i32) (br $L (i32.const 42)))
        let val = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        }));

        let br = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Break {
                name: "L",
                condition: None,
                value: Some(val),
            },
            type_: Type::UNREACHABLE,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(br);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: Some("L"),
                list,
            },
            type_: Type::I32,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(block),
        );

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = RemoveUnusedBrs;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert!(
                matches!(list[0].kind, ExpressionKind::Const(Literal::I32(42))),
                "Expected Const(42)"
            );
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_loop_back_edge_preserved() {
        let bump = Bump::new();

        // (loop $L (br $L))
        let br = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Break {
                name: "L",
                condition: None,
                value: None,
            },
            type_: Type::UNREACHABLE,
        }));

        let loop_ = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Loop {
                name: Some("L"),
                body: br,
            },
            type_: Type::UNREACHABLE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(loop_),
        );

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = RemoveUnusedBrs;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Loop { body, .. } = &body.kind {
            assert!(
                matches!(body.kind, ExpressionKind::Break { .. }),
                "Expected Break to remain"
            );
        } else {
            panic!("Expected Loop");
        }
    }

    #[test]
    fn test_nested_break() {
        let bump = Bump::new();

        // (block $A (block $B (br $A)))
        let br = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Break {
                name: "A",
                condition: None,
                value: None,
            },
            type_: Type::UNREACHABLE,
        }));

        let mut inner_list = BumpVec::new_in(&bump);
        inner_list.push(br);
        let inner = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: Some("B"),
                list: inner_list,
            },
            type_: Type::UNREACHABLE,
        }));

        let mut outer_list = BumpVec::new_in(&bump);
        outer_list.push(inner);
        let outer = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: Some("A"),
                list: outer_list,
            },
            type_: Type::NONE,
        }));

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(outer),
        );

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = RemoveUnusedBrs;
        pass.run(&mut module);

        // Br $A should be replaced by Nop
        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block {
            list: outer_list, ..
        } = &body.kind
        {
            if let ExpressionKind::Block {
                list: inner_list, ..
            } = &outer_list[0].kind
            {
                assert!(
                    matches!(inner_list[0].kind, ExpressionKind::Nop),
                    "Expected Nop in inner block"
                );
            } else {
                panic!("Expected inner Block");
            }
        } else {
            panic!("Expected outer Block");
        }
    }
}
