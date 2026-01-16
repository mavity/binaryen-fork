use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::visitor::ReadOnlyVisitor;
use std::collections::{HashMap, HashSet};

/// Call Graph representation
/// Tracks direct calls between functions.
pub struct CallGraph {
    /// Direct calls from each function (Caller -> Callees)
    pub calls: HashMap<String, HashSet<String>>,

    /// Functions that call this function (Callee -> Callers)
    pub callers: HashMap<String, HashSet<String>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            calls: HashMap::new(),
            callers: HashMap::new(),
        }
    }

    pub fn build(module: &Module) -> Self {
        let mut graph = Self::new();

        // Initialize nodes for all functions
        for func in &module.functions {
            graph.calls.insert(func.name.clone(), HashSet::new());
            graph.callers.insert(func.name.clone(), HashSet::new());
        }

        for func in &module.functions {
            if let Some(body) = &func.body {
                let caller_name = func.name.clone();
                let mut visitor = CallGraphBuilder {
                    caller: caller_name,
                    graph: &mut graph,
                };
                visitor.visit(*body);
            }
        }
        graph
    }

    /// Get all functions directly called by `func`
    pub fn get_callees(&self, func: &str) -> Option<&HashSet<String>> {
        self.calls.get(func)
    }

    /// Get all functions that directly call `func`
    pub fn get_callers(&self, func: &str) -> Option<&HashSet<String>> {
        self.callers.get(func)
    }
}

struct CallGraphBuilder<'a> {
    caller: String,
    graph: &'a mut CallGraph,
}

impl<'a, 'b> ReadOnlyVisitor<'a> for CallGraphBuilder<'b> {
    fn visit_expression(&mut self, expr: ExprRef<'a>) {
        if let ExpressionKind::Call { target, .. } = &expr.kind {
            // Record call
            let target_name = target.to_string();

            self.graph
                .calls
                .entry(self.caller.clone())
                .or_default()
                .insert(target_name.clone());

            self.graph
                .callers
                .entry(target_name)
                .or_default()
                .insert(self.caller.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind, IrBuilder};
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_call_graph_construction() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Create function "callee"
        let nop = builder.nop();
        let callee = Function::new(
            "callee".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(nop),
        );

        // Create function "caller" calling "callee"
        let call = builder.call(
            "callee",
            bumpalo::collections::Vec::new_in(&bump),
            Type::NONE,
            false,
        );
        let caller = Function::new(
            "caller".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(call),
        );

        let mut module = Module::new(&bump);
        module.add_function(callee);
        module.add_function(caller);

        let graph = CallGraph::build(&module);

        // Check "caller" -> "callee" edge
        let callees = graph.get_callees("caller").unwrap();
        assert!(callees.contains("callee"));
        assert_eq!(callees.len(), 1);

        // Check "callee" <- "caller" edge
        let callers = graph.get_callers("callee").unwrap();
        assert!(callers.contains("caller"));
        assert_eq!(callers.len(), 1);
    }
}
