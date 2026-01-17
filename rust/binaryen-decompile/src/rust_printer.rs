use binaryen_ir::Module;

/// The RustPrinter converts the annotated IR into human-readable Rust code.
pub struct RustPrinter<'m, 'a> {
    pub module: &'m Module<'a>,
    output: String,
    indent: usize,
}

impl<'m, 'a> RustPrinter<'m, 'a> {
    pub fn new(module: &'m Module<'a>) -> Self {
        Self {
            module,
            output: String::new(),
            indent: 0,
        }
    }

    pub fn print(&mut self) -> String {
        self.output.clear();
        self.output
            .push_str("// Decompiled to Rust from WebAssembly\n\n");

        for func in &self.module.functions {
            self.print_function(func);
        }

        self.output.clone()
    }

    fn print_function(&mut self, func: &binaryen_ir::Function<'a>) {
        self.output.push_str(&format!("fn {}(", func.name));
        // TODO: Print params/types properly
        self.output.push_str(") ");

        if func.results != binaryen_core::Type::NONE {
            self.output.push_str(&format!("-> {:?} ", func.results));
        }

        self.output.push_str("{\n");
        self.indent += 1;

        if let Some(body) = func.body {
            self.walk_expression(body);
        }

        self.indent -= 1;
        self.output.push_str("}\n\n");
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    "); // Rust style: 4 spaces
        }
    }

    fn walk_expression(&mut self, expr: binaryen_ir::ExprRef<'a>) {
        if self.module.annotations.is_inlined(expr) {
            return;
        }

        self.write_indent();
        // TODO: Implement skeleton for Rust syntax
        self.output
            .push_str("// TODO: Implement Rust expression printing\n");
    }
}
