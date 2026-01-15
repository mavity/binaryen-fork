use crate::module::Module;
use crate::pass::Pass;

pub struct RemoveMemoryInit;

impl Pass for RemoveMemoryInit {
    fn name(&self) -> &str {
        "RemoveMemoryInit"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // Remove all data segments
        module.data.clear();

        // Remove start function
        // Note: In C++ implementation it removes the function itself from the module.
        // Here we just unset the start pointer.
        // If we want to remove the function, we need its index/name.
        // The start field is Option<u32> (function index).
        // Removing the function would shift indices, which is complex.
        // For now, just unsetting start is safe and achieves "disable start function".
        // The function code remains but is not invoked as start.
        module.start = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExprRef, Expression, ExpressionKind};
    use crate::module::{DataSegment, Function, Module};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_remove_memory_init() {
        let bump = Bump::new();
        let mut module = Module::new(&bump);

        // Add a data segment
        let offset = Expression::const_expr(&bump, Literal::I32(0), Type::I32);
        module.add_data_segment(DataSegment {
            memory_index: 0,
            offset,
            data: vec![1, 2, 3],
        });

        // Set start function
        // We need a function first
        let func = Function::new("start".to_string(), Type::NONE, Type::NONE, vec![], None);
        module.add_function(func);
        module.set_start(0);

        // Verify state before pass
        assert!(!module.data.is_empty());
        assert!(module.start.is_some());

        // Run pass
        let mut pass = RemoveMemoryInit;
        pass.run(&mut module);

        // Verify state after pass
        assert!(module.data.is_empty(), "Data segments should be removed");
        assert!(module.start.is_none(), "Start function should be unset");

        // Function should still exist (we only removed the start reference)
        assert_eq!(module.functions.len(), 1);
    }
}
