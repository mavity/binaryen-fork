use crate::expression::{ExprRef, ExpressionKind};
use crate::module::{ExportKind, ImportKind, Module};
use crate::pass::Pass;
use crate::visitor::Visitor;
use std::collections::{HashMap, HashSet};

pub struct RemoveUnusedModuleElements;

impl Pass for RemoveUnusedModuleElements {
    fn name(&self) -> &str {
        "RemoveUnusedModuleElements"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut analyzer = Analyzer::new(module);
        analyzer.run();

        let used_funcs = analyzer.used_funcs;
        let used_globals = analyzer.used_globals;

        // Compute new indices and maps
        let mut func_remap = HashMap::new();
        let mut global_remap = HashMap::new();

        // Process functions
        // Imports are kept? Or handled separately?
        // Module struct: functions: Vec<Function>. Imports are in imports: Vec<Import>.
        // Wait, function index space includes imports first!
        // My Module struct separates imports and functions.
        // `func_idx` 0..N imports, then N..M defined functions.
        // `RemoveUnusedModuleElements` in C++ removes imports too?
        // C++: `module->removeFunctions(...)`.

        // If I remove imports, I shift defined function indices too!
        // This is getting complicated with `Module` structure.
        // `Module` has `imports` and `functions`.
        // Index space = Imports ++ Functions.

        // Let's assume we ONLY remove defined functions/globals for now to match strict "unused module elements"
        // Removing imports is `RemoveImports` pass?
        // C++ `RemoveUnusedModuleElements` removes EVERYTHING unused.

        // Step 1: Map imports (keep all for now? or analyze them too?)
        // If we analyze usage, we know which imports are used.
        // `Call` uses Name. So we can check if import name is used.
        // But indices are used in `ElementSegment`.

        // Let's track "Used Function Names" and "Used Global Names" first, then map to indices.
        // Or track "Used Indices".
        // `Call` uses Name. `GlobalGet` uses Index.
        // This mismatch is annoying.

        // Let's stick to:
        // 1. Mark used Functions (by Name).
        // 2. Mark used Globals (by Index, because `GlobalGet` uses Index).
        //    Wait, Global Index Space = Imported Globals ++ Defined Globals.

        // Re-mapping logic:
        // We need to iterate over (Imports ++ Defined) for Globals and generate new indices.

        // Let's implement index remapping properly.

        let num_func_imports = module
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Function(_, _)))
            .count();
        let num_global_imports = module
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Global(_, _)))
            .count();

        // Rebuild Imports?
        // If we remove an import, we shift indices.
        // C++ removes imports too.

        // Let's simplify: Only remove DEFINED functions and globals for this iteration.
        // Removing imports requires checking if they are used.
        // Used imports are determined by Calls (Name) and GlobalGets (Index < num_imports).

        // Analyzer should return `used_func_indices` and `used_global_indices`.
        // But for functions we have Names in Calls.
        // We can map Names to Indices before analysis?
        // Or analyze using Names for functions, and Indices for globals.

        // Let's refine Analyzer.
        let _num_global_imports = module
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Global(_, _)))
            .count();
        let num_global_imports = module.imports.len() as u32; // Assuming imports are ordered: funcs, globals...?
                                                              // Wait, imports are mixed in the vector?
                                                              // Index space:
                                                              // Function Index Space: Imported Functions, then Defined Functions.
                                                              // Global Index Space: Imported Globals, then Defined Globals.
                                                              // So we need to count specific import kinds to get the offset.
        let num_global_imports = module
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Global(_, _)))
            .count() as u32;
        let num_defined_globals = module.globals.len() as u32;

        let num_func_imports = module
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Function(_, _)))
            .count() as u32;
        let _num_defined_funcs = module.functions.len() as u32;

        // Calculate new indices for defined globals
        // Imported globals (0..num_global_imports) are kept as is (simplification)
        let mut new_global_idx = num_global_imports;
        for i in 0..num_defined_globals {
            let old_idx = num_global_imports + i;
            if used_globals.contains(&old_idx) {
                global_remap.insert(old_idx, new_global_idx);
                // We don't need to push to new_globals, we'll retain later.
                new_global_idx += 1;
            }
        }

        // For functions, we track by NAME.
        // But we need to update indices in Exports/Elements/Start.
        // We also need to remove unused functions from `module.functions`.

        // Calculate new indices for defined functions
        let mut new_func_idx = num_func_imports;

        // We can't iterate `module.functions` and remove in place easily without auxiliary structure
        // or using `retain` but we need the mapping.

        // Let's iterate and build map + new list
        // Note: `module.functions` contains defined functions.
        for (i, func) in module.functions.iter().enumerate() {
            let old_idx = num_func_imports + (i as u32);
            if used_funcs.contains(&func.name) {
                func_remap.insert(old_idx, new_func_idx);
                new_func_idx += 1;
                // We'll move it later
            }
        }

        // Phase 3: Update references

        // Update Exports
        module.exports.retain_mut(|export| {
            match export.kind {
                ExportKind::Function => {
                    if let Some(&new_idx) = func_remap.get(&export.index) {
                        export.index = new_idx;
                        true
                    } else if export.index < num_func_imports {
                        true // Keep imports
                    } else {
                        false // Remove unused export (should match used_funcs check)
                    }
                }
                ExportKind::Global => {
                    if let Some(&new_idx) = global_remap.get(&export.index) {
                        export.index = new_idx;
                        true
                    } else if export.index < num_global_imports {
                        true
                    } else {
                        false
                    }
                }
                _ => true,
            }
        });

        // Update Start
        if let Some(idx) = module.start {
            if let Some(&new_idx) = func_remap.get(&idx) {
                module.start = Some(new_idx);
            } else if idx < num_func_imports {
                // Keep
            } else {
                module.start = None;
            }
        }

        // Update Element Segments
        for elem in &mut module.elements {
            // Update func_indices
            // Also need to update offset expression? Yes.
            let mut mapper = IndexMapper {
                global_map: &global_remap,
            };
            mapper.visit(&mut elem.offset);

            elem.func_indices.retain_mut(|idx| {
                if let Some(&new_idx) = func_remap.get(idx) {
                    *idx = new_idx;
                    true
                } else if *idx < num_func_imports {
                    true
                } else {
                    false
                }
            });
        }

        // Update Data Segments
        for data in &mut module.data {
            let mut mapper = IndexMapper {
                global_map: &global_remap,
            };
            mapper.visit(&mut data.offset);
        }

        // Update Globals (Init expressions)
        for global in &mut module.globals {
            let mut mapper = IndexMapper {
                global_map: &global_remap,
            };
            mapper.visit(&mut global.init);
        }

        // Update Functions (Bodies)
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut mapper = IndexMapper {
                    global_map: &global_remap,
                };
                mapper.visit(body);
            }
        }

        // Phase 4: Remove unused items

        // Remove unused functions
        module.functions.retain(|f| used_funcs.contains(&f.name));

        // Remove unused globals
        // We can't use `retain` with index easily unless we track current index.
        let mut current_idx = num_global_imports;
        module.globals.retain(|_| {
            let keep = used_globals.contains(&current_idx);
            current_idx += 1;
            keep
        });
    }
}

struct IndexMapper<'map> {
    global_map: &'map HashMap<u32, u32>,
}

impl<'a, 'map> Visitor<'a> for IndexMapper<'map> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        // Visit children first
        self.visit_children(expr);

        match &mut expr.kind {
            ExpressionKind::GlobalGet { index } | ExpressionKind::GlobalSet { index, .. } => {
                if let Some(&new_idx) = self.global_map.get(index) {
                    *index = new_idx;
                }
                // If not in map, it must be an import (or we have a bug/dangling reference)
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExprRef, Expression, ExpressionKind, IrBuilder};
    use crate::module::{ExportKind, Function, Global, Module};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_remove_unused_functions() {
        let bump = Bump::new();
        let mut module = Module::new(&bump);

        // Func 0: Unused
        let func0 = Function::new("unused".to_string(), Type::NONE, Type::NONE, vec![], None);
        module.add_function(func0);

        // Func 1: Exported (Used)
        let func1 = Function::new("exported".to_string(), Type::NONE, Type::NONE, vec![], None);
        module.add_function(func1);
        module.export_function(1, "main".to_string()); // Index 1 because 0 is unused

        // Func 2: Called by Func 1 (Used)
        let func2 = Function::new("called".to_string(), Type::NONE, Type::NONE, vec![], None);
        module.add_function(func2);

        // Body of Func 1 calls Func 2
        let builder = IrBuilder::new(&bump);
        let call = builder.call(
            "called",
            bumpalo::collections::Vec::new_in(&bump),
            Type::NONE,
            false,
        );
        module.functions[1].body = Some(call);

        let mut pass = RemoveUnusedModuleElements;
        pass.run(&mut module);

        // Func 0 removed. Func 1 becomes 0. Func 2 becomes 1.
        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.functions[0].name, "exported");
        assert_eq!(module.functions[1].name, "called");

        // Check Export index updated
        assert_eq!(module.exports[0].index, 0);
    }

    #[test]
    fn test_remove_unused_globals() {
        let bump = Bump::new();
        let mut module = Module::new(&bump);

        // Global 0: Unused
        let init0 = Expression::const_expr(&bump, Literal::I32(0), Type::I32);
        let glob0 = Global {
            name: "unused".to_string(),
            type_: Type::I32,
            mutable: false,
            init: init0,
        };
        module.add_global(glob0);

        // Global 1: Used by Export
        let init1 = Expression::const_expr(&bump, Literal::I32(1), Type::I32);
        let glob1 = Global {
            name: "exported".to_string(),
            type_: Type::I32,
            mutable: false,
            init: init1,
        };
        module.add_global(glob1);
        module.export_global(1, "g".to_string());

        // Global 2: Used by Func 0 (which is exported)
        let init2 = Expression::const_expr(&bump, Literal::I32(2), Type::I32);
        let glob2 = Global {
            name: "used_by_code".to_string(),
            type_: Type::I32,
            mutable: false,
            init: init2,
        };
        module.add_global(glob2);

        // Func using Global 2
        let builder = IrBuilder::new(&bump);
        let get = builder.global_get(2, Type::I32);
        let func = Function::new(
            "user".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(get),
        );
        module.add_function(func);
        module.export_function(0, "user".to_string());

        let mut pass = RemoveUnusedModuleElements;
        pass.run(&mut module);

        // Global 0 removed. Global 1 -> 0. Global 2 -> 1.
        assert_eq!(module.globals.len(), 2);
        assert_eq!(module.globals[0].name, "exported");
        assert_eq!(module.globals[1].name, "used_by_code");

        // Check Export index updated
        assert_eq!(module.exports[0].name, "g");
        assert_eq!(module.exports[0].index, 0); // Was 1

        // Check Code index updated
        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::GlobalGet { index } = &body.kind {
            assert_eq!(*index, 1); // Was 2
        } else {
            panic!("Expected GlobalGet");
        }
    }
}

struct Analyzer<'a, 'b> {
    module: &'b Module<'a>,
    used_funcs: HashSet<String>,
    used_globals: HashSet<u32>,
    // Queue for reachability
    func_queue: Vec<String>,
    // We queue global indices to check their inits
    global_queue: Vec<u32>,
}

impl<'a, 'b> Analyzer<'a, 'b> {
    fn new(module: &'b Module<'a>) -> Self {
        Self {
            module,
            used_funcs: HashSet::new(),
            used_globals: HashSet::new(),
            func_queue: Vec::new(),
            global_queue: Vec::new(),
        }
    }

    fn run(&mut self) {
        // 1. Roots

        // Exports
        for export in &self.module.exports {
            match export.kind {
                ExportKind::Function => {
                    // Export uses index!
                    // We need to resolve index to name for functions.
                    if let Some(name) = self.get_func_name(export.index) {
                        self.mark_func_used(&name);
                    }
                }
                ExportKind::Global => {
                    self.mark_global_used(export.index);
                }
                _ => {} // Tables/Memories ignored for now
            }
        }

        // Start function
        if let Some(start_idx) = self.module.start {
            if let Some(name) = self.get_func_name(start_idx) {
                self.mark_func_used(&name);
            }
        }

        // Element Segments (active/passive)
        // If active, they are roots if they write to imported table?
        // For MVP, just treat all element segments as roots?
        // Or if they are used by `table.init`.
        // C++: Active segments writing to visible tables are roots.
        // Let's assume all element segments keep functions alive for now.
        for elem in &self.module.elements {
            for &func_idx in &elem.func_indices {
                if let Some(name) = self.get_func_name(func_idx) {
                    self.mark_func_used(&name);
                }
            }
        }

        // Data Segments
        // Offsets use globals.
        for data in &self.module.data {
            self.visit_expression_root(&data.offset);
        }
        for elem in &self.module.elements {
            self.visit_expression_root(&elem.offset);
        }

        // 2. Process Queues
        while !self.func_queue.is_empty() || !self.global_queue.is_empty() {
            while let Some(func_name) = self.func_queue.pop() {
                if let Some(func) = self.module.get_function(&func_name) {
                    if let Some(body) = &func.body {
                        // Visit function body
                        // We need a visitor that calls back to `self.mark_*`
                        // But we can't pass `self` to visitor easily if we are borrowing `self.module`.
                        // We can collect usages into a temp list.
                        let mut usages = UsageCollector::new();
                        usages.visit(unsafe { &mut *(body.as_ptr()) }); // Safe? ExprRef implies ownership logic but here we just read.
                                                                        // Actually `visit` expects `&mut ExprRef`.
                                                                        // But we are analyzing read-only module?
                                                                        // `Visitor` trait requires `&mut ExprRef`.
                                                                        // We cannot use mutable visitor on immutable module.
                                                                        // We need a ReadOnlyVisitor? Or just manual traversal.
                                                                        // `Visitor` trait is: `fn visit_expression(&mut self, expr: &mut ExprRef<'a>)`
                                                                        // It requires mutable expr.
                                                                        // We can't use it.

                        // We have to implement a read-only traversal.
                        self.scan_expression(body);
                    }
                }
            }

            while let Some(global_idx) = self.global_queue.pop() {
                // Find global by index
                // Index space: Imports then Defined.
                let _num_func_imports = self
                    .module
                    .imports
                    .iter()
                    .filter(|i| matches!(i.kind, ImportKind::Function(_, _)))
                    .count();
                let num_global_imports = self
                    .module
                    .imports
                    .iter()
                    .filter(|i| matches!(i.kind, ImportKind::Global(_, _)))
                    .count() as u32;

                if global_idx < num_global_imports {
                    // Imported global - no init to scan (it's external)
                } else {
                    let defined_idx = (global_idx - num_global_imports) as usize;
                    if let Some(global) = self.module.globals.get(defined_idx) {
                        self.scan_expression(&global.init);
                    }
                }
            }
        }
    }

    fn get_func_name(&self, idx: u32) -> Option<String> {
        // Resolve index to name
        let mut func_idx = 0;
        for import in &self.module.imports {
            if let ImportKind::Function(_, _) = import.kind {
                if func_idx == idx {
                    return Some(import.name.clone());
                }
                func_idx += 1;
            }
        }

        let defined_idx = idx - func_idx;
        self.module
            .functions
            .get(defined_idx as usize)
            .map(|f| f.name.clone())
    }

    fn mark_func_used(&mut self, name: &str) {
        if self.used_funcs.insert(name.to_string()) {
            self.func_queue.push(name.to_string());
        }
    }

    fn mark_global_used(&mut self, idx: u32) {
        if self.used_globals.insert(idx) {
            self.global_queue.push(idx);
        }
    }

    fn visit_expression_root(&mut self, expr: &ExprRef<'a>) {
        self.scan_expression(expr);
    }

    fn scan_expression(&mut self, expr: &ExprRef<'a>) {
        // Manual read-only traversal
        match &expr.kind {
            ExpressionKind::Call {
                target, operands, ..
            } => {
                self.mark_func_used(target);
                for op in operands {
                    self.scan_expression(op);
                }
            }
            ExpressionKind::GlobalGet { index } => {
                self.mark_global_used(*index);
            }
            ExpressionKind::GlobalSet { index, value } => {
                self.mark_global_used(*index);
                self.scan_expression(value);
            }
            // ... recurse for others ...
            ExpressionKind::Block { list, .. } => {
                for child in list {
                    self.scan_expression(child);
                }
            }
            ExpressionKind::Loop { body, .. } => self.scan_expression(body),
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.scan_expression(condition);
                self.scan_expression(if_true);
                if let Some(e) = if_false {
                    self.scan_expression(e);
                }
            }
            ExpressionKind::Unary { value, .. }
            | ExpressionKind::Drop { value }
            | ExpressionKind::LocalSet { value, .. }
            | ExpressionKind::LocalTee { value, .. }
            | ExpressionKind::Load { ptr: value, .. }
            | ExpressionKind::MemoryGrow { delta: value } => self.scan_expression(value),

            ExpressionKind::Binary { left, right, .. }
            | ExpressionKind::Store {
                ptr: left,
                value: right,
                ..
            } => {
                self.scan_expression(left);
                self.scan_expression(right);
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                self.scan_expression(condition);
                self.scan_expression(if_true);
                self.scan_expression(if_false);
            }
            ExpressionKind::CallIndirect {
                target, operands, ..
            } => {
                self.scan_expression(target);
                for op in operands {
                    self.scan_expression(op);
                }
                // Table used? usually table 0
            }
            ExpressionKind::Break {
                condition, value, ..
            } => {
                if let Some(c) = condition {
                    self.scan_expression(c);
                }
                if let Some(v) = value {
                    self.scan_expression(v);
                }
            }
            ExpressionKind::Switch {
                condition, value, ..
            } => {
                self.scan_expression(condition);
                if let Some(v) = value {
                    self.scan_expression(v);
                }
            }
            ExpressionKind::Return { value } => {
                if let Some(v) = value {
                    self.scan_expression(v);
                }
            }
            _ => {}
        }
    }
}

// Helper struct to collect usages if we needed one, but `scan_expression` works.
struct UsageCollector;
impl UsageCollector {
    fn new() -> Self {
        Self
    }
    fn visit(&mut self, _expr: &mut crate::expression::Expression) {}
}
