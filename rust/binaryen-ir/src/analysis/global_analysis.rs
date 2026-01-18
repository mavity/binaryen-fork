use crate::analysis::call_graph::CallGraph;
use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use binaryen_core::Literal;
use std::collections::{HashMap, HashSet};

pub type GlobalId = usize;
pub type FunctionId = usize;

pub struct GlobalAnalysis {
    /// Globals that are immutable (imported or constant) or effectively constant (written once at init)
    pub constant_globals: HashSet<GlobalId>,

    /// Known values for constant globals
    pub global_values: HashMap<GlobalId, Literal>,

    /// Which globals are read
    pub read_globals: HashSet<GlobalId>,

    /// Which globals are written
    pub written_globals: HashSet<GlobalId>,

    /// Which functions are reachable from exports
    pub reachable_functions: HashSet<FunctionId>,
}
/* ... existing code ... */
impl GlobalAnalysis {
    pub fn new() -> Self {
        Self {
            constant_globals: HashSet::new(),
            global_values: HashMap::new(),
            read_globals: HashSet::new(),
            written_globals: HashSet::new(),
            reachable_functions: HashSet::new(),
        }
    }

    pub fn analyze(module: &Module, call_graph: &CallGraph) -> Self {
        let mut analysis = Self::new();

        analysis.analyze_usage(module);
        analysis.analyze_globals(module);
        analysis.analyze_reachability(module, call_graph);

        analysis
    }

    fn analyze_usage(&mut self, module: &Module) {
        for func in &module.functions {
            if let Some(body) = func.body {
                self.visit_usage(body);
            }
        }
    }

    fn analyze_globals(&mut self, module: &Module) {
        // Track number of sets for each global
        let mut global_sets: HashMap<GlobalId, usize> = HashMap::new();
        for &id in &self.written_globals {
            global_sets.insert(id, 1); // We only care if it's > 0 for now in this simplified logic
        }

        // Identify constants
        for (id, global) in module.globals.iter().enumerate() {
            if !global.mutable {
                self.constant_globals.insert(id);
                // If it has an init value that is a literal, track it
                if let ExpressionKind::Const(value) = &global.init.kind {
                    self.global_values.insert(id, value.clone());
                }
            } else {
                // Mutable global. Check if it's never modified (0 sets).
                if !self.written_globals.contains(&id) {
                    self.constant_globals.insert(id);
                    if let ExpressionKind::Const(value) = &global.init.kind {
                        self.global_values.insert(id, value.clone());
                    }
                }
            }
        }
    }

    fn visit_usage(&mut self, expr: ExprRef) {
        match &expr.kind {
            ExpressionKind::GlobalGet { index } => {
                self.read_globals.insert(*index as usize);
            }
            ExpressionKind::GlobalSet { index, value } => {
                self.written_globals.insert(*index as usize);
                self.visit_usage(*value);
            }
            ExpressionKind::Block { list, .. } => {
                for child in list.iter() {
                    self.visit_usage(*child);
                }
            }
            ExpressionKind::Loop { body, .. } => self.visit_usage(*body),
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
                ..
            } => {
                self.visit_usage(*condition);
                self.visit_usage(*if_true);
                if let Some(e) = if_false {
                    self.visit_usage(*e);
                }
            }
            ExpressionKind::Binary { left, right, .. } => {
                self.visit_usage(*left);
                self.visit_usage(*right);
            }
            ExpressionKind::Unary { value, .. } => self.visit_usage(*value),
            ExpressionKind::Call { operands, .. }
            | ExpressionKind::CallIndirect { operands, .. } => {
                for op in operands.iter() {
                    self.visit_usage(*op);
                }
            }
            ExpressionKind::LocalSet { value, .. } | ExpressionKind::LocalTee { value, .. } => {
                self.visit_usage(*value);
            }
            ExpressionKind::Drop { value } => self.visit_usage(*value),
            ExpressionKind::Return { value: Some(v) } => {
                self.visit_usage(*v);
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
                ..
            } => {
                self.visit_usage(*condition);
                self.visit_usage(*if_true);
                self.visit_usage(*if_false);
            }
            _ => {}
        }
    }

    fn analyze_reachability(&mut self, module: &Module, call_graph: &CallGraph) {
        // Map function name to index
        let mut name_to_index: HashMap<&str, usize> = HashMap::new();
        for (i, func) in module.functions.iter().enumerate() {
            name_to_index.insert(&func.name, i);
        }

        // Roots: exports, start function, table elements?
        let mut roots = HashSet::new();

        // 1. Exports
        for export in &module.exports {
            if export.kind == crate::module::ExportKind::Function {
                roots.insert(export.index as usize);
            }
        }

        // 2. Start function (not modeled in Module struct currently? Assuming explicit start section or implicit main)
        // If module has start, add it.

        // Compute reachability
        let mut visited = HashSet::new();
        let mut worklist: Vec<usize> = roots.into_iter().collect();

        while let Some(func_id) = worklist.pop() {
            if !visited.contains(&func_id) {
                visited.insert(func_id);
                self.reachable_functions.insert(func_id);

                // Get function name to lookup in CallGraph
                if let Some(func) = module.functions.get(func_id) {
                    if let Some(callees) = call_graph.calls.get(&func.name) {
                        for callee_name in callees {
                            if let Some(&callee_idx) = name_to_index.get(callee_name.as_str()) {
                                if !visited.contains(&callee_idx) {
                                    worklist.push(callee_idx);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::IrBuilder;
    use crate::module::{Function, Module};
    use binaryen_core::Type;
    use bumpalo::Bump;

    #[test]
    fn test_reachability() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Func 0: Exported, calls Func 1
        let call1 = builder.call(
            "func1",
            bumpalo::collections::Vec::new_in(&bump),
            Type::NONE,
            false,
        );
        let func0 = Function::new(
            "func0".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(call1),
        );

        // Func 1: Called by Func 0, calls Func 2
        let call2 = builder.call(
            "func2",
            bumpalo::collections::Vec::new_in(&bump),
            Type::NONE,
            false,
        );
        let func1 = Function::new(
            "func1".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(call2),
        );

        // Func 2: Called by Func 1
        let nop = builder.nop();
        let func2_def = Function::new(
            "func2".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(nop),
        );

        // Func 3: Unreachable (not exported, not called)
        let nop2 = builder.nop();
        let func3 = Function::new(
            "func3".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(nop2),
        );

        let mut module = Module::new(&bump);
        module.add_function(func0);
        module.add_function(func1);
        module.add_function(func2_def);
        module.add_function(func3);

        // Export func0
        module.export_function(0, "main".to_string());

        let call_graph = CallGraph::build(&module);
        let analysis = GlobalAnalysis::analyze(&module, &call_graph);

        assert!(analysis.reachable_functions.contains(&0));
        assert!(analysis.reachable_functions.contains(&1));
        assert!(analysis.reachable_functions.contains(&2));
        assert!(!analysis.reachable_functions.contains(&3));
    }
}
