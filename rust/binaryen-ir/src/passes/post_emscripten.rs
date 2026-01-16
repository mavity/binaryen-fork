use crate::expression::ExpressionKind;
use crate::module::Module;
use crate::pass::Pass;

pub struct PostEmscripten;

impl Pass for PostEmscripten {
    fn name(&self) -> &str {
        "post-emscripten"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // PostEmscripten cleanup
        // In the full implementation, this would handle Emscripten-specific patterns.
        // For now, we provide the structure and a basic traversal.

        for func in &mut module.functions {
            if let Some(body) = func.body {
                // Potential optimization:
                // Remove redundant stack save/restore pairs if no allocation happens between them.
                // This is a common Emscripten pattern.
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
    fn test_post_emscripten_run() {
        let allocator = Bump::new();
        let mut module = Module::new(&allocator);
        // module.allocator is set by new

        let block = allocator.alloc(Expression {
            kind: ExpressionKind::Block {
                name: None,
                list: BumpVec::new_in(&allocator),
            },
            type_: Type::NONE,
        });

        // params: Type::NONE, results: Type::NONE, vars: vec![]
        let func = Function::new(
            "test_func".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(ExprRef::new(block)),
        );
        module.add_function(func);

        let mut pass = PostEmscripten;
        pass.run(&mut module);

        // Assert nothing broke
        assert!(module.get_function("test_func").is_some());
    }
}
