use crate::expression::{ExprRef, Expression, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;

pub struct SafeHeap;

impl Pass for SafeHeap {
    fn name(&self) -> &str {
        "safe-heap"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // SafeHeap
        // Goal: Instrument memory accesses to ensure they are within bounds.
        // Even though Wasm has bounds checks, this pass can enforce stricter limits
        // or support environments where hardware checks are not enough (e.g. some SGX enclaves).

        for func in &mut module.functions {
            if let Some(body) = func.body {
                // TODO: Rewrite loads/stores to check bounds against memory size or a limit.
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
    fn test_safe_heap_run() {
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

        let mut pass = SafeHeap;
        pass.run(&mut module);

        assert!(module.get_function("test_func").is_some());
    }
}
