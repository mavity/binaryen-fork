use crate::module::Module;
use crate::pass::Pass;

pub struct GenerateDynCalls;

impl Pass for GenerateDynCalls {
    fn name(&self) -> &str {
        "generate-dyncalls"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // GenerateDynCalls
        // Goal: Generate dynamic call helpers (e.g. dynCall_vi) for Emscripten.
        // This involves iterating the table and creating wrapper functions for each signature.

        // TODO: Implementation would scan table segments and generate dynCall_* exports.
        // For now, no-op or placeholder.
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
    fn test_generate_dyncalls_run() {
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

        let mut pass = GenerateDynCalls;
        pass.run(&mut module);

        assert!(module.get_function("test_func").is_some());
    }
}
