use binaryen_ir::Module;

/// The DecompilerPrinter converts the annotated IR into human-readable C-like code.
pub struct DecompilerPrinter<'m, 'a> {
    pub module: &'m Module<'a>,
    output: String,
    indent: usize,
}

impl<'m, 'a> DecompilerPrinter<'m, 'a> {
    pub fn new(module: &'m Module<'a>) -> Self {
        Self {
            module,
            output: String::new(),
            indent: 0,
        }
    }

    pub fn print(&mut self) -> String {
        self.output.clear();
        self.output.push_str("// Decompiled from WebAssembly\n\n");
        
        for func in &self.module.functions {
            self.print_function(func);
        }
        
        self.output.clone()
    }

    fn print_function(&mut self, func: &binaryen_ir::Function<'a>) {
        self.output.push_str(&format!("fn {}(", func.name));
        
        // Print params
        // For now just show the type as a string
        self.output.push_str(&format!("{:?}", func.params));
        
        self.output.push_str(") ");
        
        if func.results != binaryen_core::Type::NONE {
             self.output.push_str("-> ");
             self.output.push_str(&format!("{:?} ", func.results));
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
            self.output.push_str("  ");
        }
    }

    fn walk_expression(&mut self, expr: binaryen_ir::ExprRef<'a>) {
        self.write_indent();
        
        // Check for annotations
        if let Some(ann) = self.module.get_annotation(expr) {
            self.output.push_str(&format!("/* {:?} */ ", ann));
        }
        
        match &expr.kind {
            binaryen_ir::ExpressionKind::Binary { op, left, right } => {
                self.output.push_str(&format!("{:?}(", op));
                // We'd need to recursive print here, but for now just show refs
                self.output.push_str(&format!("{:?}, {:?}", left, right));
                self.output.push_str(")");
            }
            binaryen_ir::ExpressionKind::Const(lit) => {
                self.output.push_str(&format!("{:?}", lit));
            }
            _ => {
                self.output.push_str(&format!("{:?}", expr.kind));
            }
        }
        self.output.push_str(";\n");
    }
}
