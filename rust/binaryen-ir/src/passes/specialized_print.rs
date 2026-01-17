use crate::analysis::stats::ModuleStats;
use crate::module::Module;
use crate::pass::Pass;

/// Prints the call graph of the module.
pub struct PrintCallGraph;

impl Pass for PrintCallGraph {
    fn name(&self) -> &str {
        "PrintCallGraph"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        println!("Call Graph:");
        for func in &module.functions {
            print!("  {} ->", func.name);
            // We can use ModuleStats or a dedicated visitor to find calls
            // For now, a simple placeholder output
            println!(" [not implemented]");
        }
    }
}

/// Prints the module in a minified format (no spaces, newlines).
pub struct PrintMinified;

impl Pass for PrintMinified {
    fn name(&self) -> &str {
        "PrintMinified"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // This would use a dedicated printer. For now, we reuse the existing printer
        // but with a flag if it supported it.
        println!("Minified Output: [not implemented]");
    }
}
