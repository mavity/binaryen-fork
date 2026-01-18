use crate::analysis::call_graph::CallGraph;
use crate::analysis::global_analysis::GlobalAnalysis;
use crate::expression::{ExprRef, ExpressionKind, IrBuilder};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use std::collections::HashSet;

pub struct SimplifyGlobals;

impl Pass for SimplifyGlobals {
    fn name(&self) -> &str {
        "simplify-globals"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // 1. Build CallGraph
        let call_graph = CallGraph::build(module);

        // 2. Run Global Analysis
        let analysis = GlobalAnalysis::analyze(module, &call_graph);

        // 3. Constant Propagation
        let mut optimizer = GlobalOptimizer {
            analysis: &analysis,
            builder: IrBuilder::new(module.allocator),
        };

        for func in &mut module.functions {
            if let Some(mut body) = func.body {
                optimizer.visit(&mut body);
                func.body = Some(body);
            }
        }

        // 4. Cleanup: Remove unused or write-only globals
        let mut globals_to_remove = HashSet::new();
        for id in 0..module.globals.len() {
            if !analysis.read_globals.contains(&id) {
                // If it's not read, it's either write-only or completely unused.
                let mut is_exported = false;
                for export in &module.exports {
                    if export.kind == crate::module::ExportKind::Global
                        && export.index as usize == id
                    {
                        is_exported = true;
                        break;
                    }
                }
                if !is_exported {
                    globals_to_remove.insert(id);
                }
            }
        }

        if !globals_to_remove.is_empty() {
            self.remove_globals(module, globals_to_remove);
        }
    }
}

impl SimplifyGlobals {
    fn remove_globals<'a>(&self, module: &mut Module<'a>, to_remove: HashSet<usize>) {
        if to_remove.is_empty() {
            return;
        }

        let mut old_to_new = vec![None; module.globals.len()];
        let mut new_globals = Vec::new();
        let mut current_new_idx = 0;

        let old_globals = std::mem::take(&mut module.globals);
        for (i, global) in old_globals.into_iter().enumerate() {
            if to_remove.contains(&i) {
                old_to_new[i] = None;
            } else {
                old_to_new[i] = Some(current_new_idx);
                new_globals.push(global);
                current_new_idx += 1;
            }
        }
        module.globals = new_globals;

        // Remap indices in functions
        let mut remapper = GlobalRemapper {
            old_to_new: &old_to_new,
        };
        for func in &mut module.functions {
            if let Some(mut body) = func.body {
                remapper.visit(&mut body);
                func.body = Some(body);
            }
        }

        // Remap indices in exports
        for export in &mut module.exports {
            if export.kind == crate::module::ExportKind::Global {
                if let Some(new_idx) = old_to_new[export.index as usize] {
                    export.index = new_idx as u32;
                }
            }
        }
    }
}

struct GlobalOptimizer<'b, 'a> {
    analysis: &'b GlobalAnalysis,
    builder: IrBuilder<'a>,
}

impl<'b, 'a> Visitor<'a> for GlobalOptimizer<'b, 'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        self.visit_children(expr);

        match &mut expr.kind {
            ExpressionKind::GlobalGet { index } => {
                if let Some(literal) = self.analysis.global_values.get(&(*index as usize)) {
                    *expr = self.builder.const_(literal.clone());
                }
            }
            _ => {}
        }
    }
}

struct GlobalRemapper<'b> {
    old_to_new: &'b [Option<usize>],
}

impl<'b, 'a> Visitor<'a> for GlobalRemapper<'b> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        self.visit_children(expr);
        match &mut expr.kind {
            ExpressionKind::GlobalGet { index } => {
                if let Some(&Some(new_idx)) = self.old_to_new.get(*index as usize) {
                    *index = new_idx as u32;
                }
            }
            ExpressionKind::GlobalSet { index, .. } => {
                if let Some(&Some(new_idx)) = self.old_to_new.get(*index as usize) {
                    *index = new_idx as u32;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{Function, Global};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_propagate_constant_global() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Global 0: immutable i32 = 42
        let init = builder.const_(Literal::I32(42));
        let global = Global {
            name: "g0".to_string(),
            type_: Type::I32,
            mutable: false,
            init,
        };

        // Function: (return (global.get 0))
        let body = builder.return_(Some(builder.global_get(0, Type::I32)));
        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(body),
        );

        let mut module = Module::new(&bump);
        module.globals.push(global);
        module.functions.push(func);

        let mut pass = SimplifyGlobals;
        pass.run(&mut module);

        // Check body is now (return (i32.const 42))
        let body = module.functions[0].body.unwrap();
        if let ExpressionKind::Return { value: Some(val) } = &body.kind {
            if let ExpressionKind::Const(value) = &val.kind {
                assert_eq!(*value, Literal::I32(42));
                return;
            }
        }
        panic!("Optimization failed: expected Const(42)");
    }
}
