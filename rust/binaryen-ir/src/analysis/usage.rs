use crate::expression::{ExprRef, ExpressionKind};
use crate::module::{ImportKind, Module};
use crate::visitor::ReadOnlyVisitor;
use std::collections::{HashSet, VecDeque};

/// Tracks usage of various module elements to determine reachability.
#[derive(Debug, Default)]
pub struct UsageTracker {
    pub functions: HashSet<String>,
    pub globals: HashSet<u32>,
    pub memories: bool,
    pub tables: bool,
    pub element_segments: HashSet<u32>,
    pub data_segments: HashSet<u32>,

    pub(crate) func_queue: VecDeque<String>,
    pub(crate) global_queue: VecDeque<u32>,
}

impl UsageTracker {
    pub fn analyze(module: &Module) -> Self {
        let mut tracker = Self::default();

        // 1. Initial seeds (roots)

        // Exports
        for export in &module.exports {
            match export.kind {
                crate::module::ExportKind::Function => {
                    if let Some(name) = module_get_func_name(module, export.index) {
                        tracker.mark_func(&name);
                    }
                }
                crate::module::ExportKind::Global => {
                    tracker.mark_global(export.index);
                }
                crate::module::ExportKind::Memory => tracker.memories = true,
                crate::module::ExportKind::Table => tracker.tables = true,
            }
        }

        // Start function
        if let Some(start_idx) = module.start {
            if let Some(name) = module_get_func_name(module, start_idx) {
                tracker.mark_func(&name);
            }
        }

        // Segments
        for (i, data) in module.data.iter().enumerate() {
            let mut visitor = UsageVisitor {
                tracker: &mut tracker,
            };
            visitor.visit(data.offset);
            // If the segment is active, it's a seed if the memory is used?
            // Actually, in Binaryen, we keep all active segments once memory is used.
            // For now, let's just mark the globals/functions they use.
            tracker.data_segments.insert(i as u32);
        }

        for (i, elem) in module.elements.iter().enumerate() {
            let mut visitor = UsageVisitor {
                tracker: &mut tracker,
            };
            visitor.visit(elem.offset);
            for &func_idx in &elem.func_indices {
                if let Some(name) = module_get_func_name(module, func_idx) {
                    tracker.mark_func(&name);
                }
            }
            tracker.element_segments.insert(i as u32);
        }

        // 2. Transitive closure (worklist)
        while !tracker.func_queue.is_empty() || !tracker.global_queue.is_empty() {
            while let Some(func_name) = tracker.func_queue.pop_front() {
                if let Some(func) = module.get_function(&func_name) {
                    if let Some(body) = func.body {
                        let mut visitor = UsageVisitor {
                            tracker: &mut tracker,
                        };
                        visitor.visit(body);
                    }
                }
            }

            while let Some(global_idx) = tracker.global_queue.pop_front() {
                if let Some(global) = module_get_global(module, global_idx) {
                    let mut visitor = UsageVisitor {
                        tracker: &mut tracker,
                    };
                    visitor.visit(global.init);
                }
            }
        }

        tracker
    }

    fn mark_func(&mut self, name: &str) {
        if self.functions.insert(name.to_string()) {
            self.func_queue.push_back(name.to_string());
        }
    }

    fn mark_global(&mut self, index: u32) {
        if self.globals.insert(index) {
            self.global_queue.push_back(index);
        }
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

fn module_get_global<'a, 'b>(
    module: &'b Module<'a>,
    index: u32,
) -> Option<&'b crate::module::Global<'a>> {
    let mut current_idx = 0;
    for import in &module.imports {
        if let ImportKind::Global(_, _) = import.kind {
            current_idx += 1;
        }
    }

    if index < current_idx {
        None
    } else {
        module.globals.get((index - current_idx) as usize)
    }
}

struct UsageVisitor<'a> {
    tracker: &'a mut UsageTracker,
}

impl<'a, 'b> ReadOnlyVisitor<'b> for UsageVisitor<'a> {
    fn visit_expression(&mut self, expr: ExprRef<'b>) {
        match &expr.kind {
            ExpressionKind::Call { target, .. } => {
                self.tracker.mark_func(target);
            }
            ExpressionKind::GlobalGet { index } | ExpressionKind::GlobalSet { index, .. } => {
                self.tracker.mark_global(*index);
            }
            ExpressionKind::Load { .. }
            | ExpressionKind::Store { .. }
            | ExpressionKind::AtomicRMW { .. }
            | ExpressionKind::AtomicCmpxchg { .. }
            | ExpressionKind::AtomicWait { .. }
            | ExpressionKind::AtomicNotify { .. }
            | ExpressionKind::MemoryCopy { .. }
            | ExpressionKind::MemoryFill { .. }
            | ExpressionKind::MemorySize
            | ExpressionKind::MemoryGrow { .. } => {
                self.tracker.memories = true;
            }
            ExpressionKind::MemoryInit { segment, .. } | ExpressionKind::DataDrop { segment } => {
                self.tracker.memories = true;
                self.tracker.data_segments.insert(*segment);
            }
            ExpressionKind::TableInit {
                table: _, segment, ..
            }
            | ExpressionKind::ElemDrop { segment } => {
                self.tracker.tables = true;
                self.tracker.element_segments.insert(*segment);
            }
            ExpressionKind::CallIndirect { .. }
            | ExpressionKind::TableGet { .. }
            | ExpressionKind::TableSet { .. }
            | ExpressionKind::TableSize { .. }
            | ExpressionKind::TableGrow { .. }
            | ExpressionKind::TableFill { .. }
            | ExpressionKind::TableCopy { .. } => {
                self.tracker.tables = true;
            }
            ExpressionKind::RefFunc { func } => {
                self.tracker.mark_func(func);
            }
            _ => {}
        }
    }
}
