use binaryen_ir::Module;

/// The CPrinter converts the annotated IR into human-readable C-like code.
pub struct CPrinter<'m, 'a> {
    pub module: &'m Module<'a>,
    output: String,
    indent: usize,
}

impl<'m, 'a> CPrinter<'m, 'a> {
    pub fn new(module: &'m Module<'a>) -> Self {
        Self {
            module,
            output: String::new(),
            indent: 0,
        }
    }

    fn get_local_name(&self, index: u32, expr: Option<binaryen_ir::ExprRef<'a>>) -> String {
        if let Some(e) = expr {
            if let Some(name) = self.module.annotations.get_local_name(e) {
                return name.to_string();
            }
        }
        format!("p{}", index)
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
        // Skip if marked as inlined (as a statement)
        if self.module.annotations.is_inlined(expr) {
            return;
        }

        self.write_indent();

        match &expr.kind {
            binaryen_ir::ExpressionKind::Block { name, list, .. } => {
                let mut is_if = false;
                if let Some((condition, inverted)) = self.module.annotations.get_if_info(expr) {
                    is_if = true;
                    self.output.push_str("if (");
                    if inverted {
                        self.output.push_str("!");
                    }
                    self.walk_inline_expression(condition);
                    self.output.push_str(") ");
                } else if let Some(n) = name {
                    self.output.push_str(&format!("{}: ", n));
                }

                self.output.push_str("{\n");
                self.indent += 1;

                let start_idx = if is_if { 1 } else { 0 };
                for &child in &list[start_idx..] {
                    self.walk_expression(child);
                }

                self.indent -= 1;
                self.write_indent();
                self.output.push_str("}\n");
                return;
            }
            binaryen_ir::ExpressionKind::LocalSet { index, value } => {
                let name = self.get_local_name(*index, Some(expr));
                self.output.push_str(&format!("{} = ", name));
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
                use binaryen_ir::annotation::LoopType;
                match self.module.annotations.get_loop_type(expr) {
                    Some(LoopType::DoWhile) => loop_keyword = "do-while",
                    Some(LoopType::While) => loop_keyword = "while",
                    _ => {}
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
        use binaryen_ir::ExpressionKind;

        // Check for inlined value
        if let Some(val) = self.module.annotations.get_inlined_value(expr) {
            self.walk_inline_expression(val);
            return;
        }

        match &expr.kind {
            ExpressionKind::Binary { op, left, right } => {
                self.output.push_str("(");
                self.walk_inline_expression(*left);
                let op_str = match op {
                    binaryen_ir::BinaryOp::AddInt32 | binaryen_ir::BinaryOp::AddInt64 => " + ",
                    binaryen_ir::BinaryOp::SubInt32 | binaryen_ir::BinaryOp::SubInt64 => " - ",
                    binaryen_ir::BinaryOp::MulInt32 | binaryen_ir::BinaryOp::MulInt64 => " * ",
                    binaryen_ir::BinaryOp::DivSInt32 | binaryen_ir::BinaryOp::DivSInt64 => " / ",
                    binaryen_ir::BinaryOp::EqInt32 | binaryen_ir::BinaryOp::EqInt64 => " == ",
                    binaryen_ir::BinaryOp::NeInt32 | binaryen_ir::BinaryOp::NeInt64 => " != ",
                    binaryen_ir::BinaryOp::LtSInt32 | binaryen_ir::BinaryOp::LtSInt64 => " < ",
                    binaryen_ir::BinaryOp::LeSInt32 | binaryen_ir::BinaryOp::LeSInt64 => " <= ",
                    binaryen_ir::BinaryOp::GtSInt32 | binaryen_ir::BinaryOp::GtSInt64 => " > ",
                    binaryen_ir::BinaryOp::GeSInt32 | binaryen_ir::BinaryOp::GeSInt64 => " >= ",
                    binaryen_ir::BinaryOp::AndInt32 | binaryen_ir::BinaryOp::AndInt64 => " & ",
                    binaryen_ir::BinaryOp::OrInt32 | binaryen_ir::BinaryOp::OrInt64 => " | ",
                    binaryen_ir::BinaryOp::XorInt32 | binaryen_ir::BinaryOp::XorInt64 => " ^ ",
                    binaryen_ir::BinaryOp::ShlInt32 | binaryen_ir::BinaryOp::ShlInt64 => " << ",
                    binaryen_ir::BinaryOp::ShrSInt32 | binaryen_ir::BinaryOp::ShrSInt64 => " >> ",
                    binaryen_ir::BinaryOp::ShrUInt32 | binaryen_ir::BinaryOp::ShrUInt64 => " >>> ",
                    _ => " <OP> ",
                };
                self.output.push_str(op_str);
                self.walk_inline_expression(*right);
                self.output.push_str(")");
            }
            ExpressionKind::Unary { op, value } => {
                let op_prefix = match op {
                    binaryen_ir::UnaryOp::ExtendSInt32 => "(i64)(i32)",
                    binaryen_ir::UnaryOp::ExtendUInt32 => "(i64)(u32)",
                    binaryen_ir::UnaryOp::WrapInt64 => "(i32)",
                    binaryen_ir::UnaryOp::ExtendS8Int32 => "(i32)(i8)",
                    binaryen_ir::UnaryOp::ExtendS16Int32 => "(i32)(i16)",
                    binaryen_ir::UnaryOp::ExtendS8Int64 => "(i64)(i8)",
                    binaryen_ir::UnaryOp::ExtendS16Int64 => "(i64)(i16)",
                    binaryen_ir::UnaryOp::ExtendS32Int64 => "(i64)(i32)",
                    _ => "",
                };

                if !op_prefix.is_empty() {
                    self.output.push_str(op_prefix);
                    self.walk_inline_expression(*value);
                } else {
                    self.output.push_str(&format!("{:?}(", op));
                    self.walk_inline_expression(*value);
                    self.output.push_str(")");
                }
            }
            ExpressionKind::LocalGet { index } => {
                let name = self.get_local_name(*index, Some(expr));
                self.output.push_str(&name);
            }
            ExpressionKind::LocalSet { index, value } => {
                let name = self.get_local_name(*index, Some(expr));
                self.output.push_str(&format!("({} = ", name));
                self.walk_inline_expression(*value);
                self.output.push_str(")");
            }
            ExpressionKind::LocalTee { index, value } => {
                let name = self.get_local_name(*index, Some(expr));
                self.output.push_str(&format!("({} = ", name));
                self.walk_inline_expression(*value);
                self.output.push_str(")");
            }
            ExpressionKind::GlobalGet { index } => {
                self.output.push_str(&format!("g{}", index));
            }
            ExpressionKind::GlobalSet { index, value } => {
                self.output.push_str(&format!("(g{} = ", index));
                self.walk_inline_expression(*value);
                self.output.push_str(")");
            }
            ExpressionKind::Const(lit) => {
                let s = match lit {
                    binaryen_core::Literal::I32(v) => v.to_string(),
                    binaryen_core::Literal::I64(v) => v.to_string(),
                    binaryen_core::Literal::F32(v) => v.to_string(),
                    binaryen_core::Literal::F64(v) => v.to_string(),
                    _ => format!("{:?}", lit),
                };
                self.output.push_str(&s);
            }
            ExpressionKind::Load { ptr, .. } => {
                use binaryen_ir::annotation::HighLevelType;
                if self.module.annotations.get_high_level_type(*ptr) == Some(HighLevelType::Pointer)
                {
                    self.output.push_str("*(");
                } else {
                    self.output.push_str("Load(");
                }
                self.walk_inline_expression(*ptr);
                self.output.push_str(")");
            }
            ExpressionKind::Store { ptr, value, .. } => {
                self.output.push_str("Store(");
                self.walk_inline_expression(*ptr);
                self.output.push_str(", ");
                self.walk_inline_expression(*value);
                self.output.push_str(")");
            }
            ExpressionKind::Call {
                target, operands, ..
            } => {
                self.output.push_str(&format!("{}(", target));
                for (i, op) in operands.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.walk_inline_expression(*op);
                }
                self.output.push_str(")");
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                self.output.push_str("(");
                self.walk_inline_expression(*condition);
                self.output.push_str(" ? ");
                self.walk_inline_expression(*if_true);
                self.output.push_str(" : ");
                self.walk_inline_expression(*if_false);
                self.output.push_str(")");
            }
            ExpressionKind::Drop { value } => {
                self.output.push_str("Drop(");
                self.walk_inline_expression(*value);
                self.output.push_str(")");
            }
            ExpressionKind::Nop => {
                self.output.push_str("nop");
            }
            ExpressionKind::Unreachable => {
                self.output.push_str("unreachable");
            }
            _ => {
                self.output.push_str(&format!("{:?}", expr.kind));
            }
        }
    }
}
