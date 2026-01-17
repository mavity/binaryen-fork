use crate::analysis::stats::ModuleStats;
use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use std::collections::HashMap;

/// Reorders types in the type section based on usage frequency.
pub struct ReorderTypes;

impl Pass for ReorderTypes {
    fn name(&self) -> &str {
        "ReorderTypes"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        if module.types.is_empty() {
            return;
        }

        let stats = ModuleStats::collect(module);
        // Wait, ModuleStats currently doesn't count type_idx usage comprehensively.
        // Let's assume we have it or use a simplified count.

        let mut type_counts = HashMap::new();
        for func in &module.functions {
            if let Some(idx) = func.type_idx {
                *type_counts.entry(idx).or_insert(0) += 1;
            }
        }

        // Also count types in CallIndirect
        let mut visitor = TypeUsageCounter {
            counts: &mut type_counts,
        };
        for func in &module.functions {
            if let Some(body) = func.body {
                visitor.visit_readonly(body);
            }
        }

        let mut sorted_indices: Vec<usize> = (0..module.types.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            let count_a = type_counts.get(&(a as u32)).copied().unwrap_or(0);
            let count_b = type_counts.get(&(b as u32)).copied().unwrap_or(0);
            count_b.cmp(&count_a)
        });

        let mut remap = HashMap::new();
        let mut new_types = Vec::with_capacity(module.types.len());

        for (new_idx, &old_idx) in sorted_indices.iter().enumerate() {
            remap.insert(old_idx as u32, new_idx as u32);
            new_types.push(module.types[old_idx].clone());
        }

        module.types = new_types;

        // Update references
        for func in &mut module.functions {
            if let Some(idx) = &mut func.type_idx {
                if let Some(&new_id) = remap.get(idx) {
                    *idx = new_id;
                }
            }
        }

        let mut updater = TypeUpdater { remap: &remap };
        for func in &mut module.functions {
            if let Some(mut body) = func.body {
                updater.visit(&mut body);
            }
        }
    }
}

struct TypeUsageCounter<'a> {
    counts: &'a mut HashMap<u32, usize>,
}

impl<'a, 'b> TypeUsageCounter<'a> {
    fn visit_readonly(&mut self, expr: ExprRef<'b>) {
        if let ExpressionKind::CallIndirect { type_, .. } = &expr.kind {
            if let Some(id) = type_.signature_id() {
                *self.counts.entry(id).or_insert(0) += 1;
            }
        }
        expr.kind.for_each_child(|child| self.visit_readonly(child));
    }
}

struct TypeUpdater<'a> {
    remap: &'a HashMap<u32, u32>,
}

impl<'a, 'b> Visitor<'b> for TypeUpdater<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'b>) {
        self.visit_children(expr);
        if let ExpressionKind::CallIndirect { type_, .. } = &mut expr.kind {
            if let Some(id) = type_.signature_id() {
                if let Some(&new_id) = self.remap.get(&id) {
                    *type_ = binaryen_core::Type::from_signature_id(new_id);
                }
            }
        }
    }
}
