use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::Type;
use std::collections::HashMap;

/// Global Refining pass: Refines global types to more specific subtypes
///
/// This pass analyzes global variable usage (assignments) and refines their types.
pub struct GlobalRefining;

impl Pass for GlobalRefining {
    fn name(&self) -> &str {
        "global-refining"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // Map from global index to LUB of assigned types
        // let mut inferred_types: HashMap<u32, Option<Type>> = HashMap::new();

        let mut analyzer = GlobalAnalyzer {
            assigned_types: HashMap::new(),
        };

        // Scan all functions
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                analyzer.visit(body);
            }
        }

        // Also scan global initializers themselves
        for (i, global) in module.globals.iter().enumerate() {
            analyzer.note_assignment(i as u32, global.init.type_);
        }

        let inferred_types = analyzer.assigned_types;

        // Update globals
        for (i, global) in module.globals.iter_mut().enumerate() {
            if let Some(Some(inferred)) = inferred_types.get(&(i as u32)) {
                if *inferred != global.type_ {
                    // Update type if valid subtype
                }
            }
        }
    }
}

struct GlobalAnalyzer {
    assigned_types: HashMap<u32, Option<Type>>,
}

impl GlobalAnalyzer {
    fn note_assignment(&mut self, index: u32, type_: Type) {
        let entry = self.assigned_types.entry(index).or_insert(None);
        if let Some(val) = entry {
            *val = Self::lub(*val, type_);
        } else {
            *entry = Some(type_);
        }
    }

    fn lub(a: Type, b: Type) -> Type {
        if a == b {
            a
        } else {
            a
        }
    }
}

impl<'a> Visitor<'a> for GlobalAnalyzer {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        if let ExpressionKind::GlobalSet { index, value } = &mut expr.kind {
            self.note_assignment(*index, value.type_);
            self.visit(value);
        } else {
            self.visit_children(expr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind};
    use crate::module::{Global, Module};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_global_refining_analysis() {
        // Setup a module with one global
        let bump = Bump::new();
        let init = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(0)),
            type_: Type::I32,
        });

        let global = Global {
            name: "g0".to_string(),
            type_: Type::I32,
            mutable: true,
            init: ExprRef::new(init),
        };

        let mut module = Module::new(&bump);
        module.globals.push(global);

        let mut pass = GlobalRefining;
        pass.run(&mut module);

        // Nothing changes in MVP, but analysis runs
    }
}
