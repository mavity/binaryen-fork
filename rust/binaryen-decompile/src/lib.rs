pub mod lifter;
pub mod printer;
pub mod passes;

pub use lifter::Lifter;
pub use printer::DecompilerPrinter;

/// High-level entry point for decompilation.
pub struct Decompiler<'a> {
    pub module: &'a mut binaryen_ir::Module<'a>,
}

impl<'a> Decompiler<'a> {
    pub fn new(module: &'a mut binaryen_ir::Module<'a>) -> Self {
        Self { module }
    }

    /// Run the lifting passes to identify high-level constructs.
    pub fn lift(&mut self) {
        let mut lifter = Lifter::new();
        lifter.run(self.module);
    }

    /// Print the decompiled code.
    pub fn decompile(&self) -> String {
        let mut printer = DecompilerPrinter::new(self.module);
        printer.print()
    }
}
