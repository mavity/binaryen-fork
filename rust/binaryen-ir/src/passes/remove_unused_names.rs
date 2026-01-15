use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use std::collections::{HashMap, HashSet};

pub struct RemoveUnusedNames;

impl Pass for RemoveUnusedNames {
    fn name(&self) -> &str {
        "RemoveUnusedNames"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut analyzer = NameAnalyzer::new();
                analyzer.visit(body);
            }
        }
    }
}

struct NameAnalyzer {
    // Track which names are actually targeted by branches
    branches_seen: HashMap<String, HashSet<usize>>,
    // Counter for unique expression IDs to distinguish branches
    expr_counter: usize,
}

impl NameAnalyzer {
    fn new() -> Self {
        Self {
            branches_seen: HashMap::new(),
            expr_counter: 0,
        }
    }

    fn get_expr_id(&mut self) -> usize {
        let id = self.expr_counter;
        self.expr_counter += 1;
        id
    }

    fn note_branch_target(&mut self, name: &str) {
        let id = self.get_expr_id();
        self.branches_seen
            .entry(name.to_string())
            .or_insert_with(HashSet::new)
            .insert(id);
    }
}

impl<'a> Visitor<'a> for NameAnalyzer {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        // First, collect branch targets from this expression
        match &expr.kind {
            ExpressionKind::Break { name, .. } => {
                if !name.is_empty() {
                    self.note_branch_target(name);
                }
            }
            ExpressionKind::Switch { names, default, .. } => {
                for name in names {
                    if !name.is_empty() {
                        self.note_branch_target(name);
                    }
                }
                if !default.is_empty() {
                    self.note_branch_target(default);
                }
            }
            _ => {}
        }

        // Visit children
        self.visit_children(expr);

        // After visiting children, process blocks to remove unused names
        match &mut expr.kind {
            ExpressionKind::Block { name, .. } => {
                if let Some(block_name) = name {
                    // Check if this block name was ever targeted
                    if !self.branches_seen.contains_key(*block_name) {
                        // No branches to this block - remove the name
                        *name = None;
                    } else {
                        // Name was used, remove it from the map (optimally, we could keep it if there are outer blocks with same name, but shadowing is handled by unique names usually)
                        // Actually, if we remove it, we might affect outer blocks if they have the same name.
                        // But binaryen guarantees unique names usually or handles shadowing.
                        // For this pass, we just check if it was seen.
                        // To handle nested names correctly we should arguably pop the usage, but `branches_seen` accumulates all branches.
                        // If we have:
                        // (block $l (block $l (br $l)))
                        // The br $l targets the inner block.
                        // This pass is simple and assumes unique names or that we don't clear the map.
                        // Clearing the map `self.branches_seen.remove` might be incorrect if multiple blocks share a name and both are targeted?
                        // But if multiple blocks share a name, the break targets the nearest one.
                        // So the inner one "consumes" the break.
                        // So removing it from `branches_seen` IS correct for the nearest scoping.

                        self.branches_seen.remove(*block_name);
                    }
                }
            }
            ExpressionKind::Loop { name, .. } => {
                if let Some(loop_name) = name {
                    if !self.branches_seen.contains_key(*loop_name) {
                        *name = None;
                    } else {
                        self.branches_seen.remove(*loop_name);
                    }
                }
            }
            _ => {}
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
    fn test_remove_unused_block_name() {
        let bump = Bump::new();

        // Create a block with a name that's never branched to
        let const_expr = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(const_expr);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: Some("unused"),
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

        let mut pass = RemoveUnusedNames;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { name, .. } = &body.kind {
            assert!(name.is_none(), "Expected block name to be removed");
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_preserve_used_block_name() {
        let bump = Bump::new();

        // Create a block with a name that IS branched to
        let br_expr = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Break {
                name: "target",
                condition: None,
                value: None,
            },
            type_: Type::UNREACHABLE,
        }));

        let const_expr = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        }));

        let mut list = BumpVec::new_in(&bump);
        list.push(br_expr);
        list.push(const_expr);

        let block = ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block {
                name: Some("target"),
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

        let mut pass = RemoveUnusedNames;
        pass.run(&mut module);

        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { name, .. } = &body.kind {
            assert!(name.is_some(), "Expected block name to be preserved");
            assert_eq!(name.unwrap(), "target");
        } else {
            panic!("Expected Block");
        }
    }
}
