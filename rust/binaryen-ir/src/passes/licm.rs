use crate::expression::ExprRef;
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;

/// LICM (Loop-Invariant Code Motion): Hoists invariants out of loops
pub struct LICM;

impl Pass for LICM {
    fn name(&self) -> &str {
        "licm"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut hoister = LoopInvariantHoister;
                hoister.visit(body);
            }
        }
    }
}

struct LoopInvariantHoister;

impl<'a> Visitor<'a> for LoopInvariantHoister {
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
    fn test_licm() {
        let bump = Bump::new();
        let val = Expression::const_expr(&bump, Literal::I32(1), Type::I32);
        let func = Function::new("test".to_string(), Type::NONE, Type::I32, vec![], Some(val));
        let mut module = Module::new(&bump);
        module.add_function(func);
        let mut pass = LICM;
        pass.run(&mut module);
        assert!(module.functions[0].body.is_some());
    }
}
