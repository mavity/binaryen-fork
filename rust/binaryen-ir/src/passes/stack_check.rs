use crate::expression::{ExprRef, Expression, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;

pub struct StackCheck;

impl Pass for StackCheck {
    fn name(&self) -> &str {
        "stack-check"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // StackCheck
        // Goal: Add checks at function entry to prevent stack overflow.
        // Requires importing a stack check function (e.g. from env) or checking against a global.

        // TODO: iterate functions and prepend check logic to body.

        for func in &mut module.functions {
            if let Some(body) = func.body {
                // Prepend stack check.
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
    fn test_stack_check_run() {
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

        let mut pass = StackCheck;
        pass.run(&mut module);

        assert!(module.get_function("test_func").is_some());
    }
}
