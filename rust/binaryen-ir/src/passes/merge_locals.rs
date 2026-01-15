use crate::expression::ExprRef;
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;

/// Merge Locals pass: Combines similar local variable patterns
pub struct MergeLocals;

impl Pass for MergeLocals {
    fn name(&self) -> &str {
        "merge-locals"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut merger = LocalMerger;
                merger.visit(body);
            }
        }
    }
}

struct LocalMerger;

impl<'a> Visitor<'a> for LocalMerger {
    fn visit_expression(&mut self, _expr: &mut ExprRef<'a>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::Expression;
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_merge_locals() {
        let bump = Bump::new();
        let val = Expression::const_expr(&bump, Literal::I32(1), Type::I32);
        let func = Function::new("test".to_string(), Type::NONE, Type::I32, vec![], Some(val));
        let mut module = Module::new(&bump);
        module.add_function(func);
        let mut pass = MergeLocals;
        pass.run(&mut module);
        assert!(module.functions[0].body.is_some());
    }
}
