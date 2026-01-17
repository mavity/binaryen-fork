use crate::passes::IdentifyBooleans;
use binaryen_ir::Module;

/// The Lifter is responsible for running passes that "lift" low-level WASM IR
/// into high-level constructs by populating annotations on Expressions.
pub struct Lifter;

impl Lifter {
    pub fn new() -> Self {
        Self
    }

    pub fn run<'a>(&mut self, module: &mut Module<'a>) {
        // Run boolean identification
        let mut id_bools = IdentifyBooleans::new();
        id_bools.run(module);

        // TODO: Register and run other lifting passes here.
        // 2. Loop structure identification (for/while/do)
        // 3. Variable role identification (induction variables, etc.)
        // 4. Condition lifting
    }
}
