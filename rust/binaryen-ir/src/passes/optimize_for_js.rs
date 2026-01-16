use crate::expression::{Expression, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use binaryen_core::Type;

pub struct OptimizeForJS;

impl Pass for OptimizeForJS {
    fn name(&self) -> &str {
        "optimize-for-js"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // OptimizeForJS
        // Goal: Optimize operations for JS execution (e.g., prefer i32 over i64 where possible).

        for func in &mut module.functions {
            if let Some(body) = func.body {
                // TODO: Implement optimization logic.
                // Traverse body and replace eligible i64 ops with i32.
            }
        }
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
    fn test_optimize_for_js_run() {
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

        let mut pass = OptimizeForJS;
        pass.run(&mut module);

        assert!(module.get_function("test_func").is_some());
    }
}
