use crate::passes::{
    ExpressionRecombination, IdentifyBooleans, IdentifyIfElse, IdentifyLoops, IdentifyPointers,
    NameInferencePass,
};
use binaryen_ir::Module;

/// The Lifter is responsible for running passes that "lift" low-level WASM IR
/// into high-level constructs by populating annotations on Expressions.
pub struct Lifter;

impl Lifter {
    pub fn new() -> Self {
        Self
    }

    pub fn run<'a>(&mut self, module: &mut Module<'a>) {
        // 1. Identify types first
        IdentifyPointers::new().run(module);
        IdentifyBooleans::new().run(module);

        // 2. Identify control flow structures
        IdentifyLoops::new().run(module);
        IdentifyIfElse::new().run(module);

        // 3. Recombine expressions (inlining single-use locals)
        ExpressionRecombination::run(module);

        // 4. Infer variable names
        NameInferencePass::new().run(module);

        // TODO: Register and run other lifting passes here.
        // 4. Condition lifting
    }
}
