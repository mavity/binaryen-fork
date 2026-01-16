use crate::expression::{ExprRef, Expression, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use binaryen_core::Type;

pub struct I64ToI32Lowering;

impl Pass for I64ToI32Lowering {
    fn name(&self) -> &str {
        "i64-to-i32-lowering"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // I64ToI32Lowering
        // Goal: Lower 64-bit integers to 32-bit integers for environments without i64 support (like old JS).

        // This is a global transformation that affects function signatures, locals, and instructions.

        for func in &mut module.functions {
            // TODO:
            // 1. Lower params and results (i64 -> i32, i32)
            // 2. Lower locals
            // 3. Lower instructions (add, sub, mul, etc.)

            if let Some(body) = func.body {
                // For now, we perform a no-op traversal or simplified check.
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
    fn test_i64_to_i32_lowering_run() {
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

        let mut pass = I64ToI32Lowering;
        pass.run(&mut module);

        assert!(module.get_function("test_func").is_some());
    }
}
