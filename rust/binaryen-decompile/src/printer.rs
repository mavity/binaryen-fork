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
        use binaryen_ir::annotation::Annotation;

        // Skip if marked as inlined (as a statement)
        if matches!(self.module.get_annotation(expr), Some(Annotation::Inlined)) {
            return;
        }

        self.write_indent();

        match &expr.kind {
            binaryen_ir::ExpressionKind::Block { name, list, .. } => {
                if let Some(n) = name {
                    self.output.push_str(&format!("{}: ", n));
                }
                self.output.push_str("{\n");
                self.indent += 1;
                for &child in list {
                    self.walk_expression(child);
                }
                self.indent -= 1;
                self.write_indent();
                self.output.push_str("}\n");
                return; // Blocks don't need a semicolon here usually
            }
            binaryen_ir::ExpressionKind::LocalSet { index, value } => {
                self.output.push_str(&format!("p{} = ", index));
                self.walk_inline_expression(*value);
            }
            binaryen_ir::ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.output.push_str("if (");
                self.walk_inline_expression(*condition);
                self.output.push_str(") ");
                self.walk_expression(*if_true);
                if let Some(f) = if_false {
                    self.write_indent();
                    self.output.push_str("else ");
                    self.walk_expression(*f);
                }
                return;
            }
            binaryen_ir::ExpressionKind::Loop { name, body } => {
                let mut loop_keyword = "loop";
                if let Some(ann) = self.module.get_annotation(expr) {
                    if matches!(
                        ann,
                        Annotation::Loop(binaryen_ir::annotation::LoopType::DoWhile)
                    ) {
                        loop_keyword = "do-while";
                    } else if matches!(
                        ann,
                        Annotation::Loop(binaryen_ir::annotation::LoopType::While)
                    ) {
                        loop_keyword = "while";
                    }
                }
                self.output
                    .push_str(&format!("{} {} ", loop_keyword, name.unwrap_or("unnamed")));
                self.walk_expression(*body);
                return;
            }
            _ => {
                // For other things that are expressions used as statements
                self.walk_inline_expression(expr);
            }
        }
        self.output.push_str(";\n");
    }

    fn walk_inline_expression(&mut self, expr: binaryen_ir::ExprRef<'a>) {
        use binaryen_ir::annotation::Annotation;

        // Check for inlined value
        if let Some(Annotation::InlinedValue(val)) = self.module.get_annotation(expr) {
            self.walk_inline_expression(*val);
            return;
        }

        match &expr.kind {
            binaryen_ir::ExpressionKind::Binary { op, left, right } => {
                self.output.push_str("(");
                self.walk_inline_expression(*left);
                self.output.push_str(&format!(" {:?} ", op));
                self.walk_inline_expression(*right);
                self.output.push_str(")");
            }
            binaryen_ir::ExpressionKind::LocalGet { index } => {
                self.output.push_str(&format!("p{}", index));
            }
            binaryen_ir::ExpressionKind::Const(lit) => {
                self.output.push_str(&format!("{:?}", lit));
            }
            binaryen_ir::ExpressionKind::Load { ptr, .. } => {
                if let Some(Annotation::Type(binaryen_ir::annotation::HighLevelType::Pointer)) =
                    self.module.get_annotation(*ptr)
                {
                    self.output.push_str("*(");
                } else {
                    self.output.push_str("Load(");
                }
                self.walk_inline_expression(*ptr);
                self.output.push_str(")");
            }
            _ => {
                self.output.push_str(&format!("{:?}", expr.kind));
            }
        }
    }
}
