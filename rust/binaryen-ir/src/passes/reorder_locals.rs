use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use std::collections::HashMap;

/// Reorders locals within each function to group them by type (for better compression).
pub struct ReorderLocals;

impl Pass for ReorderLocals {
    fn name(&self) -> &str {
        "ReorderLocals"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if func.vars.is_empty() {
                continue;
            }

            // 1. Identify all locals (params + vars)
            // WASM parameters cannot be reordered without changing signature,
            // so we only reorder 'vars'.

            let num_params = match func.params.is_tuple() {
                // Simplified param count logic - in a real impl we'd query TypeStore
                true => 2, // Placeholder
                false => {
                    if func.params == binaryen_core::Type::NONE {
                        0
                    } else {
                        1
                    }
                }
            };

            // In our Module IR, vars is a Vec<Type>.
            // We want to sort them by type.
            let mut indexed_vars: Vec<(usize, binaryen_core::Type)> =
                func.vars.iter().cloned().enumerate().collect();
            indexed_vars.sort_by_key(|&(_, ty)| ty);

            let mut remap = HashMap::new();
            let mut new_vars = Vec::with_capacity(func.vars.len());

            for (new_idx, (old_idx, ty)) in indexed_vars.into_iter().enumerate() {
                let old_local_idx = (num_params + old_idx) as u32;
                let new_local_idx = (num_params + new_idx) as u32;
                remap.insert(old_local_idx, new_local_idx);
                new_vars.push(ty);
            }

            func.vars = new_vars;

            // 2. Update all local.get/set/tee in the body
            if let Some(mut body) = func.body {
                let mut updater = LocalRemapper { remap: &remap };
                updater.visit(&mut body);
            }
        }
    }
}

struct LocalRemapper<'a> {
    remap: &'a HashMap<u32, u32>,
}

impl<'a, 'b> Visitor<'b> for LocalRemapper<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'b>) {
        self.visit_children(expr);
        match &mut expr.kind {
            ExpressionKind::LocalGet { index }
            | ExpressionKind::LocalSet { index, .. }
            | ExpressionKind::LocalTee { index, .. } => {
                if let Some(&new_idx) = self.remap.get(index) {
                    *index = new_idx;
                }
            }
            _ => {}
        }
    }
}
