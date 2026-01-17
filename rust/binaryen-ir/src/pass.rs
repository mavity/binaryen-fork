use crate::module::Module;
use crate::validation::Validator;

pub trait Pass {
    fn name(&self) -> &str;
    fn run<'a>(&mut self, module: &mut Module<'a>);
}

#[derive(Debug, Clone)]
pub struct OptimizationOptions {
    pub debug: bool,
    pub validate: bool,
    pub validate_globally: bool,
    pub optimize_level: u32,
    pub shrink_level: u32,
    pub traps_never_happen: bool,
    pub low_memory_unused: bool,
    pub fast_math: bool,
    pub zero_filled_memory: bool,
    pub closed_world: bool,
    pub debug_info: bool,
}

impl Default for OptimizationOptions {
    fn default() -> Self {
        Self {
            debug: false,
            validate: true,
            validate_globally: true,
            optimize_level: 0,
            shrink_level: 0,
            traps_never_happen: false,
            low_memory_unused: false,
            fast_math: false,
            zero_filled_memory: false,
            closed_world: false,
            debug_info: false,
        }
    }
}

impl OptimizationOptions {
    pub fn o0() -> Self {
        Self {
            optimize_level: 0,
            shrink_level: 0,
            ..Default::default()
        }
    }

    pub fn o1() -> Self {
        Self {
            optimize_level: 1,
            shrink_level: 0,
            ..Default::default()
        }
    }

    pub fn o2() -> Self {
        Self {
            optimize_level: 2,
            shrink_level: 0,
            ..Default::default()
        }
    }

    pub fn o3() -> Self {
        Self {
            optimize_level: 3,
            shrink_level: 0,
            ..Default::default()
        }
    }

    pub fn o4() -> Self {
        Self {
            optimize_level: 4,
            shrink_level: 0,
            ..Default::default()
        }
    }

    pub fn os() -> Self {
        Self {
            optimize_level: 2,
            shrink_level: 1,
            ..Default::default()
        }
    }

    pub fn oz() -> Self {
        Self {
            optimize_level: 2,
            shrink_level: 2,
            ..Default::default()
        }
    }
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

    /// The main entry point for -O1, -O2, etc.
    /// Ported from C++ PassRunner::addDefaultOptimizationPasses.
    pub fn add_default_optimization_passes(&mut self, options: &OptimizationOptions) {
        if options.optimize_level == 0 && options.shrink_level == 0 {
            return; // -O0: no optimizations
        }

        // Global pre-passes
        self.add_global_pre_passes(options);

        // Function-level optimizations (the "meat")
        self.add_function_optimization_passes(options);

        // Global post-passes
        self.add_global_post_passes(options);
    }

    fn add_global_pre_passes(&mut self, options: &OptimizationOptions) {
        self.add(crate::passes::duplicate_function_elimination::DuplicateFunctionElimination);
        if options.optimize_level >= 2 {
            self.add(crate::passes::remove_unused_module_elements::RemoveUnusedModuleElements);
        }
    }

    fn add_function_optimization_passes(&mut self, options: &OptimizationOptions) {
        self.add(crate::passes::dce::DCE);
        self.add(crate::passes::remove_unused_names::RemoveUnusedNames);
        self.add(crate::passes::remove_unused_brs::RemoveUnusedBrs);
        self.add(crate::passes::optimize_instructions::OptimizeInstructions::new());

        if options.optimize_level >= 2 || options.shrink_level >= 2 {
            self.add(crate::passes::pick_load_signs::PickLoadSigns);
        }

        self.add(crate::passes::precompute::Precompute);

        if options.optimize_level >= 2 || options.shrink_level >= 2 {
            self.add(crate::passes::code_pushing::CodePushing);
        }

        self.add(crate::passes::simplify_locals::SimplifyLocals::with_options(true, false, true));
        self.add(crate::passes::vacuum::Vacuum);

        self.add(crate::passes::coalesce_locals::CoalesceLocals);
        self.add(crate::passes::vacuum::Vacuum);

        self.add(crate::passes::merge_blocks::MergeBlocks);
        self.add(crate::passes::remove_unused_brs::RemoveUnusedBrs);
        self.add(crate::passes::remove_unused_names::RemoveUnusedNames);
        self.add(crate::passes::merge_blocks::MergeBlocks);

        self.add(crate::passes::precompute::Precompute);
        self.add(crate::passes::optimize_instructions::OptimizeInstructions::new());
        self.add(crate::passes::vacuum::Vacuum);
    }

    fn add_global_post_passes(&mut self, options: &OptimizationOptions) {
        if options.optimize_level >= 2 || options.shrink_level >= 1 {
            self.add(crate::passes::dae_optimizing::DaeOptimizing);
        }
        if options.optimize_level >= 2 || options.shrink_level >= 2 {
            self.add(crate::passes::inlining::Inlining);
        }
        self.add(crate::passes::duplicate_function_elimination::DuplicateFunctionElimination);
    }

    /// Bundle: Standard cleanup sequence (vacuum + name removal + local simplification)
    pub fn add_cleanup_passes(&mut self) {
        self.add(crate::passes::vacuum::Vacuum);
        self.add(crate::passes::remove_unused_names::RemoveUnusedNames);
        self.add(crate::passes::simplify_locals::SimplifyLocals::new());
    }

    /// Bundle: Dead Code Elimination sequence
    pub fn add_dead_code_elimination_passes(&mut self) {
        self.add(crate::passes::dce::DCE);
        self.add(crate::passes::remove_unused_module_elements::RemoveUnusedModuleElements);
    }

    /// Bundle: Branch optimization (merge blocks + remove unused branches)
    pub fn add_branch_optimization_passes(&mut self) {
        self.add(crate::passes::merge_blocks::MergeBlocks);
        self.add(crate::passes::remove_unused_brs::RemoveUnusedBrs);
    }

    pub fn get_all_pass_names() -> Vec<&'static str> {
        vec![
            "dce",
            "vacuum",
            "remove-unused-names",
            "remove-unused-brs",
            "remove-unused-module-elements",
            "simplify-locals",
            "simplify-locals-notee",
            "simplify-locals-nostructure",
            "simplify-locals-notee-nostructure",
            "coalesce-locals",
            "reorder-locals",
            "merge-blocks",
            "precompute",
            "optimize-instructions",
            "pick-load-signs",
            "code-pushing",
            "duplicate-function-elimination",
            "inlining",
            "dae-optimizing",
        ]
    }

    pub fn add_by_name(&mut self, name: &str) -> bool {
        match name {
            "dce" => self.add(crate::passes::dce::DCE),
            "vacuum" => self.add(crate::passes::vacuum::Vacuum),
            "remove-unused-names" => {
                self.add(crate::passes::remove_unused_names::RemoveUnusedNames)
            }
            "remove-unused-brs" => self.add(crate::passes::remove_unused_brs::RemoveUnusedBrs),
            "remove-unused-module-elements" => {
                self.add(crate::passes::remove_unused_module_elements::RemoveUnusedModuleElements)
            }
            "simplify-locals" => self.add(crate::passes::simplify_locals::SimplifyLocals::new()),
            "simplify-locals-notee" => self.add(
                crate::passes::simplify_locals::SimplifyLocals::with_options(false, true, true),
            ),
            "simplify-locals-nostructure" => self.add(
                crate::passes::simplify_locals::SimplifyLocals::with_options(true, false, true),
            ),
            "simplify-locals-notee-nostructure" => self.add(
                crate::passes::simplify_locals::SimplifyLocals::with_options(false, false, true),
            ),
            "coalesce-locals" => self.add(crate::passes::coalesce_locals::CoalesceLocals),
            "reorder-locals" => self.add(crate::passes::merge_locals::MergeLocals), // Using MergeLocals as placeholder if reorder-locals is not yet implemented or same
            "merge-blocks" => self.add(crate::passes::merge_blocks::MergeBlocks),
            "precompute" => self.add(crate::passes::precompute::Precompute),
            "optimize-instructions" => {
                self.add(crate::passes::optimize_instructions::OptimizeInstructions::new())
            }
            "pick-load-signs" => self.add(crate::passes::pick_load_signs::PickLoadSigns),
            "code-pushing" => self.add(crate::passes::code_pushing::CodePushing),
            "duplicate-function-elimination" => self
                .add(crate::passes::duplicate_function_elimination::DuplicateFunctionElimination),
            "inlining" => self.add(crate::passes::inlining::Inlining),
            "dae-optimizing" => self.add(crate::passes::dae_optimizing::DaeOptimizing),
            _ => return false,
        }
        true
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
                if let Some(func) = module.functions.get_mut(0) {
                    func.results = Type::F32;
                }
            }
        }

        let bump = Bump::new();
        let body = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        });

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(ExprRef::new(body)),
        );

        let bump_module = bumpalo::Bump::new();
        let mut module = Module::new(&bump_module);
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

    #[test]
    fn test_optimization_options_presets() {
        let o0 = OptimizationOptions::o0();
        assert_eq!(o0.optimize_level, 0);
        assert_eq!(o0.shrink_level, 0);

        let o3 = OptimizationOptions::o3();
        assert_eq!(o3.optimize_level, 3);
        assert_eq!(o3.shrink_level, 0);

        let os = OptimizationOptions::os();
        assert_eq!(os.optimize_level, 2);
        assert_eq!(os.shrink_level, 1);

        let oz = OptimizationOptions::oz();
        assert_eq!(oz.optimize_level, 2);
        assert_eq!(oz.shrink_level, 2);
    }
}
