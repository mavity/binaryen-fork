use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::Type;
use std::collections::HashMap;

/// Local Subtyping pass: Refines local types to more specific subtypes
///
/// This pass analyzes local variable usage and refines their types
/// to be more specific where possible.
///
/// Algorithm:
/// 1. Analyze all `local.set` and `local.tee` instructions to find the types of values assigned to each local.
/// 2. Calculate the Least Upper Bound (LUB) of all assigned types for each local.
/// 3. If the calculated LUB is a proper subtype of the declared type, update the local's type.
pub struct LocalSubtyping;

impl Pass for LocalSubtyping {
    fn name(&self) -> &str {
        "local-subtyping"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            // Map from local index to the LUB of types assigned to it
            // Start with None (meaning "no values assigned yet" or "bottom")
            let mut inferred_types: HashMap<u32, Option<Type>> = HashMap::new();

            // Also need to track if a local is read before written?
            // Standard analysis: If read, it must have default value or be initialized.
            // Parameters are initialized. Locals are initialized to default (0/null).
            // So the "initial" type is the type of the default value.
            // BUT `LocalSubtyping` usually ignores the default initialization if it can prove it's overwritten before read,
            // or it includes the default type if not.
            // For simplicity in this port, we will scan assignments.

            // Note: `func.params` are not mutable in type usually, only `func.vars` (locals).
            // Params indices: 0 .. params.len()-1
            // Vars indices: params.len() .. params.len()+vars.len()-1

            let param_count = if func.params == Type::NONE { 0 } else { 1 }; // Simplified param count
            let var_start = param_count;

            if let Some(body) = &mut func.body {
                let mut analyzer = LocalAnalyzer {
                    assigned_types: HashMap::new(),
                };
                analyzer.visit(body);
                inferred_types = analyzer.assigned_types;
            }

            // Update vars
            for (i, var_type) in func.vars.iter_mut().enumerate() {
                let local_index = (var_start + i) as u32;

                // If we found assignments, the LUB is in inferred_types.
                // If no assignments found, the local keeps its default value (0), so effectively it stores 'Type' (or subtype of 0?).
                // Actually if never assigned, it is only 0. 0 is I32/I64/etc.
                // If never assigned, we can't really change the type unless we know it's only read as something else?
                // No, we refine based on what is PUT into it.

                if let Some(Some(inferred)) = inferred_types.get(&local_index) {
                    if *inferred != *var_type && inferred.is_subtype_of(*var_type) {
                        *var_type = *inferred;
                    }
                }
            }
        }
    }
}

struct LocalAnalyzer {
    /// Map local index -> LUB of assigned types
    assigned_types: HashMap<u32, Option<Type>>,
}

impl LocalAnalyzer {
    fn note_assignment(&mut self, index: u32, type_: Type) {
        let entry = self.assigned_types.entry(index).or_insert(None);
        if let Some(val) = entry {
            *val = Type::get_lub(*val, type_);
        } else {
            *entry = Some(type_);
        }
    }
}

impl<'a> Visitor<'a> for LocalAnalyzer {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        match &mut expr.kind {
            ExpressionKind::LocalSet { index, value } => {
                self.note_assignment(*index, value.type_);
            }
            ExpressionKind::LocalTee { index, value } => {
                self.note_assignment(*index, value.type_);
            }
            _ => {}
        }
        self.visit_children(expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::IrBuilder;
    use crate::module::{Function, Module};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_local_subtyping_noop_mvp() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (local.set 0 (i32.const 42))
        let c42 = builder.const_(Literal::I32(42));
        let set = builder.local_set(0, c42);

        // Function has local 0 as I32
        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32], // Local 0
            Some(set),
        );

        let bump_mod = Bump::new();
        let mut module = Module::new(&bump_mod);
        module.add_function(func);

        let mut pass = LocalSubtyping;
        pass.run(&mut module);

        // Should remain I32
        assert_eq!(module.functions[0].vars[0], Type::I32);
    }
}
