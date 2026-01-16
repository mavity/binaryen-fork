use crate::module::{ExportKind, Module};
use crate::pass::Pass;
use binaryen_core::Type;

pub struct LegalizeJSInterface;

impl Pass for LegalizeJSInterface {
    fn name(&self) -> &str {
        "legalize-js-interface"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // LegalizeJSInterface
        // Goal: Ensure import/export types are JS-compatible (e.g. split i64).

        // Simple check for now: just iterate exports to find illegal types.
        // Full implementation would create wrapper functions.

        let mut _illegal_exports = Vec::new();

        for export in &module.exports {
            if export.kind == ExportKind::Function {
                if let Some(func) = module.get_function(&export.name) {
                    // Wait, export.name is export name, verify if it matches internal name?
                    // module.exports stores internal name? No, export.name is exported name.
                    // Export struct has index? module.exports definition:
                    // pub struct Export { pub name: String, pub kind: ExportKind, pub index: u32 }
                    // So index points to function index.

                    if let Some(func) = module.functions.get(export.index as usize) {
                        if func.params == Type::I64 || func.results == Type::I64 {
                            _illegal_exports.push(export.name.clone());
                        }
                    }
                }
            }
        }

        // TODO: transformation logic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExprRef, Expression, ExpressionKind};
    use crate::module::Function;
    use binaryen_core::Type;
    use bumpalo::collections::Vec as BumpVec;
    use bumpalo::Bump;

    #[test]
    fn test_legalize_js_interface_run() {
        let allocator = Bump::new();
        let mut module = Module::new(&allocator);

        let block = allocator.alloc(Expression {
            kind: ExpressionKind::Block {
                name: None,
                list: BumpVec::new_in(&allocator),
            },
            type_: Type::NONE,
        });

        let func = Function::new(
            "test_func".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(ExprRef::new(block)),
        );
        module.add_function(func);
        module.export_function(0, "test_func".to_string());

        let mut pass = LegalizeJSInterface;
        pass.run(&mut module);

        assert!(module.get_function("test_func").is_some());
    }
}
