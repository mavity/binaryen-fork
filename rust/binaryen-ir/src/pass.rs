use crate::module::Module;
use crate::validation::Validator;

pub trait Pass {
    fn name(&self) -> &str;
    fn run<'a>(&mut self, module: &mut Module<'a>);
}

pub struct PassRunner {
    passes: Vec<Box<dyn Pass>>,
    validate_after_pass: bool,
}

impl Default for PassRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl PassRunner {
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            validate_after_pass: false,
        }
    }

    pub fn set_validate_globally(&mut self, validate: bool) {
        self.validate_after_pass = validate;
    }

    pub fn add<P: Pass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    pub fn run<'a>(&mut self, module: &mut Module<'a>) {
        for pass in &mut self.passes {
            pass.run(module);

            if self.validate_after_pass {
                let validator = Validator::new(module);
                let (valid, errors) = validator.validate();
                if !valid {
                    let err_msg = errors.join("\n");
                    panic!(
                        "Validation failed after pass '{}':\n{}",
                        pass.name(),
                        err_msg
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::Function;
    use binaryen_core::Type;

    struct MockPass;

    impl Pass for MockPass {
        fn name(&self) -> &str {
            "MockPass"
        }

        fn run<'a>(&mut self, module: &mut Module<'a>) {
            for func in &mut module.functions {
                func.name.push_str("_visited");
            }
        }
    }

    #[test]
    fn test_pass_runner() {
        let bump = bumpalo::Bump::new();
        let mut module = Module::new(&bump);
        module.add_function(Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            None,
        ));

        let mut runner = PassRunner::new();
        runner.add(MockPass);
        runner.run(&mut module);

        assert_eq!(module.functions[0].name, "test_visited");
    }

    #[test]
    fn test_pass_runner_validation_failure() {
        use crate::expression::{ExprRef, Expression, ExpressionKind};
        use binaryen_core::Literal;
        use bumpalo::Bump;

        struct BrokenPass;
        impl Pass for BrokenPass {
            fn name(&self) -> &str {
                "BrokenPass"
            }
            fn run<'a>(&mut self, module: &mut Module<'a>) {
                // Break module validity: Change function return type but not body
                // Function initially expects I32 and has I32 body.
                // We change expected return to F32.
                if let Some(func) = module.functions.get_mut(0) {
                    func.results = Type::F32;
                }
            }
        }

        let bump = Bump::new();
        // Body: (i32.const 42)
        let body = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        });

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32, // Correct
            vec![],
            Some(ExprRef::new(body)),
        );

        let bump = bumpalo::Bump::new();
        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut runner = PassRunner::new();
        runner.set_validate_globally(true);
        runner.add(BrokenPass);

        // This should panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runner.run(&mut module);
        }));

        assert!(
            result.is_err(),
            "PassRunner should panic on validation error"
        );
    }
}
