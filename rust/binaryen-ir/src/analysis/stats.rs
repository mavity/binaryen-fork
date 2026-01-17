use crate::expression::{ExprRef, ExpressionKind};
use crate::module::{ImportKind, Module};
use crate::visitor::ReadOnlyVisitor;
use std::collections::HashMap;

/// Collects statistics about module elements to aid in reorganization.
#[derive(Debug, Default)]
pub struct ModuleStats {
    pub function_counts: HashMap<String, usize>,
    pub global_counts: HashMap<u32, usize>,
}

impl ModuleStats {
    pub fn collect(module: &Module) -> Self {
        let mut stats = Self::default();

        // Count references in exports
        for export in &module.exports {
            match export.kind {
                crate::module::ExportKind::Function => {
                    if let Some(name) = module_get_func_name(module, export.index) {
                        stats.increment_func(&name);
                    }
                }
                crate::module::ExportKind::Global => {
                    stats.increment_global(export.index);
                }
                _ => {}
            }
        }

        // Count references in start function
        if let Some(start_idx) = module.start {
            if let Some(name) = module_get_func_name(module, start_idx) {
                stats.increment_func(&name);
            }
        }

        // Count references in table elements
        for elem in &module.elements {
            for &idx in &elem.func_indices {
                if let Some(name) = module_get_func_name(module, idx) {
                    stats.increment_func(&name);
                }
            }
        }

        // Count references in function bodies
        for func in &module.functions {
            if let Some(body) = func.body {
                let mut visitor = StatsVisitor { stats: &mut stats };
                visitor.visit(body);
            }
        }

        // Count references in global init expressions
        for global in &module.globals {
            let mut visitor = StatsVisitor { stats: &mut stats };
            visitor.visit(global.init);
        }

        stats
    }

    fn increment_func(&mut self, name: &str) {
        *self.function_counts.entry(name.to_string()).or_insert(0) += 1;
    }

    fn increment_global(&mut self, index: u32) {
        *self.global_counts.entry(index).or_insert(0) += 1;
    }
}

fn module_get_func_name(module: &Module, index: u32) -> Option<String> {
    let mut current_idx = 0;
    for import in &module.imports {
        if let ImportKind::Function(_, _) = import.kind {
            if current_idx == index {
                return Some(import.name.clone());
            }
            current_idx += 1;
        }
    }

    let defined_idx = index - current_idx;
    module
        .functions
        .get(defined_idx as usize)
        .map(|f| f.name.clone())
}

struct StatsVisitor<'a> {
    stats: &'a mut ModuleStats,
}

impl<'a, 'b> ReadOnlyVisitor<'b> for StatsVisitor<'a> {
    fn visit_expression(&mut self, expr: ExprRef<'b>) {
        match &expr.kind {
            ExpressionKind::Call { target, .. } | ExpressionKind::RefFunc { func: target } => {
                self.stats.increment_func(target);
            }
            ExpressionKind::GlobalGet { index } | ExpressionKind::GlobalSet { index, .. } => {
                self.stats.increment_global(*index);
            }
            _ => {}
        }
        self.visit_children(expr);
    }
}
