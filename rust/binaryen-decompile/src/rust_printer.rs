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

    fn get_local_name(
        &self,
        func: &binaryen_ir::Function<'a>,
        index: u32,
        expr: Option<binaryen_ir::ExprRef<'a>>,
    ) -> String {
        if (index as usize) < func.local_names.len() && !func.local_names[index as usize].is_empty()
        {
            return func.local_names[index as usize].clone();
        }
        if let Some(e) = expr {
            if let Some(name) = self.module.annotations.get_local_name(e) {
                return name.to_string();
            }
        }
        format!("v{}", index)
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

        let param_types = func.params.tuple_elements();
        for (i, ty) in param_types.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            let name = if i < func.local_names.len() && !func.local_names[i].is_empty() {
                &func.local_names[i]
            } else {
                "var"
            };
            self.output.push_str(&format!("{}: {:?}", name, ty));
        }

        self.output.push_str(") ");

        if func.results != binaryen_core::Type::NONE {
            self.output.push_str("-> ");
            self.output.push_str(&format!("{:?} ", func.results));
        }

        self.output.push_str("{\n");
        self.indent += 1;

        if let Some(body) = func.body {
            if let binaryen_ir::ExpressionKind::Block { list, .. } = &body.kind {
                for (i, &child) in list.iter().enumerate() {
                    let is_last = i == list.len() - 1;
                    self.walk_expression_ext(
                        func,
                        child,
                        is_last && func.results != binaryen_core::Type::NONE,
                    );
                }
            } else {
                self.walk_expression_ext(func, body, func.results != binaryen_core::Type::NONE);
            }
        }

        self.indent -= 1;
        self.output.push_str("}\n\n");
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn walk_expression_ext(
        &mut self,
        func: &binaryen_ir::Function<'a>,
        expr: binaryen_ir::ExprRef<'a>,
        is_return: bool,
    ) {
        if self.module.annotations.is_inlined(expr) {
            return;
        }

        self.write_indent();

        match &expr.kind {
            binaryen_ir::ExpressionKind::Block { name, list, .. } => {
                let mut is_if = false;
                if let Some((condition, inverted)) = self.module.annotations.get_if_info(expr) {
                    is_if = true;
                    self.output.push_str("if ");
                    if inverted {
                        self.output.push_str("!");
                    }
                    self.walk_inline_expression(func, condition);
                    self.output.push_str(" ");
                } else if let Some(n) = name {
                    self.output.push_str(&format!("'{}: ", n));
                }

                self.output.push_str("{\n");
                self.indent += 1;

                let start_idx = if is_if { 1 } else { 0 };
                for (i, &child) in list.iter().enumerate().skip(start_idx) {
                    let child_is_last = i == list.len() - 1;
                    self.walk_expression_ext(func, child, is_return && child_is_last);
                }

                self.indent -= 1;
                self.write_indent();
                self.output.push_str("}\n");
                return;
            }
            binaryen_ir::ExpressionKind::LocalSet { index, value } => {
                let name = self.get_local_name(func, *index, Some(expr));
                self.output.push_str(&format!("{} = ", name));
                self.walk_inline_expression(func, *value);
            }
            binaryen_ir::ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.output.push_str("if ");
                self.walk_inline_expression(func, *condition);
                self.output.push_str(" ");
                self.walk_expression_ext(func, *if_true, is_return);
                if let Some(f) = if_false {
                    self.write_indent();
                    self.output.push_str("else ");
                    self.walk_expression_ext(func, *f, is_return);
                }
                return;
            }
            binaryen_ir::ExpressionKind::Loop { name, body } => {
                use binaryen_ir::annotation::LoopType;
                let loop_keyword = match self.module.annotations.get_loop_type(expr) {
                    Some(LoopType::While) => "while ",
                    Some(LoopType::DoWhile) => "loop ",
                    _ => "loop ",
                };
                if let Some(n) = name {
                    self.output.push_str(&format!("'{}: ", n));
                }
                self.output.push_str(loop_keyword);
                self.walk_expression_ext(func, *body, false);
                return;
            }
            _ => {
                self.walk_inline_expression(func, expr);
            }
        }

        if is_return {
            self.output.push_str("\n");
        } else {
            self.output.push_str(";\n");
        }
    }

    fn walk_inline_expression(
        &mut self,
        func: &binaryen_ir::Function<'a>,
        expr: binaryen_ir::ExprRef<'a>,
    ) {
        use binaryen_ir::ExpressionKind;

        if let Some(val) = self.module.annotations.get_inlined_value(expr) {
            self.walk_inline_expression(func, val);
            return;
        }

        match &expr.kind {
            ExpressionKind::Binary { op, left, right } => {
                self.output.push_str("(");
                self.walk_inline_expression(func, *left);
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
                    binaryen_ir::BinaryOp::ShrUInt32 | binaryen_ir::BinaryOp::ShrUInt64 => " >> ", // logical shift
                    _ => " <OP> ",
                };
                self.output.push_str(op_str);
                self.walk_inline_expression(func, *right);
                self.output.push_str(")");
            }
            ExpressionKind::Unary { op, value } => {
                let (prefix, suffix) = match op {
                    binaryen_ir::UnaryOp::ExtendSInt32 => ("(", " as i64)"),
                    binaryen_ir::UnaryOp::ExtendUInt32 => ("(", " as u64)"),
                    binaryen_ir::UnaryOp::WrapInt64 => ("(", " as i32)"),
                    _ => ("", ""),
                };

                if !prefix.is_empty() {
                    self.output.push_str(prefix);
                    self.walk_inline_expression(func, *value);
                    self.output.push_str(suffix);
                } else {
                    self.output.push_str(&format!("{:?}(", op));
                    self.walk_inline_expression(func, *value);
                    self.output.push_str(")");
                }
            }
            ExpressionKind::LocalGet { index } => {
                let name = self.get_local_name(func, *index, Some(expr));
                self.output.push_str(&name);
            }
            ExpressionKind::LocalSet { index, value }
            | ExpressionKind::LocalTee { index, value } => {
                let name = self.get_local_name(func, *index, Some(expr));
                self.output.push_str(&format!("{{ {} = ", name));
                self.walk_inline_expression(func, *value);
                self.output.push_str("; }"); // In Rust, assignments are () or we use block
            }
            ExpressionKind::GlobalGet { index } => {
                self.output.push_str(&format!("G{}", index));
            }
            ExpressionKind::GlobalSet { index, value } => {
                self.output.push_str(&format!("{{ G{} = ", index));
                self.walk_inline_expression(func, *value);
                self.output.push_str("; }");
            }
            ExpressionKind::Const(lit) => {
                let s = match lit {
                    binaryen_core::Literal::I32(v) => v.to_string(),
                    binaryen_core::Literal::I64(v) => format!("{}i64", v),
                    binaryen_core::Literal::F32(v) => format!("{}f32", v),
                    binaryen_core::Literal::F64(v) => format!("{}f64", v),
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
                    self.output.push_str("read_mem(");
                }
                self.walk_inline_expression(func, *ptr);
                self.output.push_str(")");
            }
            ExpressionKind::Store { ptr, value, .. } => {
                self.output.push_str("write_mem(");
                self.walk_inline_expression(func, *ptr);
                self.output.push_str(", ");
                self.walk_inline_expression(func, *value);
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
                    self.walk_inline_expression(func, *op);
                }
                self.output.push_str(")");
            }
            _ => {
                self.output.push_str(&format!("{:?}", expr.kind));
            }
        }
    }
}
