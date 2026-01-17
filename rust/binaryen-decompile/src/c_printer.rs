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

    fn type_name(&self, ty: binaryen_core::Type) -> String {
        use binaryen_core::Type;
        match ty {
            Type::NONE => "void".to_string(),
            Type::I32 => "int32_t".to_string(),
            Type::I64 => "int64_t".to_string(),
            Type::F32 => "float".to_string(),
            Type::F64 => "double".to_string(),
            _ => format!("{:?}", ty),
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
        let return_type = self.type_name(func.results);
        self.output
            .push_str(&format!("{} {}(", return_type, func.name));

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
            self.output
                .push_str(&format!("{} {}", self.type_name(*ty), name));
        }

        self.output.push_str(") {\n");
        self.indent += 1;

        // Declare locals
        let param_count = func.params.tuple_elements().len();
        for (i, ty) in func.vars.iter().enumerate() {
            let local_idx = param_count + i;
            let name = self.get_local_name(func, local_idx as u32, None);
            self.write_indent();
            self.output
                .push_str(&format!("{} {} = 0;\n", self.type_name(*ty), name));
        }

        if let Some(body) = func.body {
            match &body.kind {
                binaryen_ir::ExpressionKind::Block {
                    name: None, list, ..
                } if self.module.annotations.get_if_info(body).is_none() => {
                    // Skip extra braces for anonymous top-level blocks that aren't lifted
                    for (i, &child) in list.iter().enumerate() {
                        let is_last = i == list.len() - 1;
                        self.walk_expression_ext(
                            func,
                            child,
                            is_last && func.results != binaryen_core::Type::NONE,
                        );
                    }
                }
                _ => {
                    self.walk_expression_ext(func, body, func.results != binaryen_core::Type::NONE);
                }
            }
        }

        self.indent -= 1;
        self.output.push_str("}\n\n");
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    fn walk_expression_ext(
        &mut self,
        func: &binaryen_ir::Function<'a>,
        expr: binaryen_ir::ExprRef<'a>,
        is_return: bool,
    ) {
        // Skip if marked as inlined (as a statement)
        if self.module.annotations.is_inlined(expr) {
            return;
        }

        self.write_indent();

        if is_return {
            match &expr.kind {
                binaryen_ir::ExpressionKind::Block { .. }
                | binaryen_ir::ExpressionKind::If { .. }
                | binaryen_ir::ExpressionKind::Loop { .. } => {
                    // These constructs handle is_return internally by passing it to their children
                }
                _ => {
                    self.output.push_str("return ");
                }
            }
        }

        match &expr.kind {
            binaryen_ir::ExpressionKind::Block { name, list, .. } => {
                let mut is_if = false;
                if let Some((condition, inverted)) = self.module.annotations.get_if_info(expr) {
                    is_if = true;
                    self.output.push_str("if (");
                    if inverted {
                        self.output.push_str("!");
                    }
                    self.walk_inline_expression(func, condition);
                    self.output.push_str(") ");
                }

                self.output.push_str("{\n");
                self.indent += 1;

                let start_idx = if is_if { 1 } else { 0 };
                for (i, &child) in list.iter().enumerate().skip(start_idx) {
                    let is_last = i == list.len() - 1;
                    self.walk_expression_ext(func, child, is_return && is_last);
                }

                self.indent -= 1;
                self.write_indent();
                self.output.push_str("}\n");

                if let Some(n) = name {
                    self.output.push_str(&format!("{}: ;\n", n));
                }
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
                self.output.push_str("if (");
                self.walk_inline_expression(func, *condition);
                self.output.push_str(") ");
                self.walk_expression_ext(func, *if_true, is_return);
                if let Some(f) = if_false {
                    self.write_indent();
                    self.output.push_str("else ");
                    self.walk_expression_ext(func, *f, is_return);
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
                self.walk_expression_ext(func, *body, false);
                return;
            }
            binaryen_ir::ExpressionKind::Return { value } => {
                if is_return {
                    if let Some(val) = value {
                        self.walk_inline_expression(func, *val);
                    }
                } else {
                    if let Some(val) = value {
                        self.output.push_str("return ");
                        self.walk_inline_expression(func, *val);
                    } else {
                        self.output.push_str("return");
                    }
                }
            }
            binaryen_ir::ExpressionKind::Break {
                name,
                condition,
                value,
            } => {
                if let Some(cond) = condition {
                    self.output.push_str("if (");
                    self.walk_inline_expression(func, *cond);
                    self.output.push_str(") { ");
                }

                if let Some(val) = value {
                    self.output
                        .push_str(&format!("/* break {} with value */ ", name));
                    // We don't have a good way to return values from labels in C
                    self.walk_inline_expression(func, *val);
                    self.output.push_str("; ");
                }
                self.output.push_str(&format!("goto {}", name));

                if condition.is_some() {
                    self.output.push_str("; }");
                }
            }
            _ => {
                // For other things that are expressions used as statements
                self.walk_inline_expression(func, expr);
            }
        }
        self.output.push_str(";\n");
    }

    fn walk_inline_expression(
        &mut self,
        func: &binaryen_ir::Function<'a>,
        expr: binaryen_ir::ExprRef<'a>,
    ) {
        use binaryen_ir::ExpressionKind;

        // Check for inlined value
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
                    binaryen_ir::BinaryOp::RemSInt32 | binaryen_ir::BinaryOp::RemSInt64 => " % ",
                    binaryen_ir::BinaryOp::ShlInt32 | binaryen_ir::BinaryOp::ShlInt64 => " << ",
                    binaryen_ir::BinaryOp::ShrSInt32 | binaryen_ir::BinaryOp::ShrSInt64 => " >> ",
                    binaryen_ir::BinaryOp::ShrUInt32 | binaryen_ir::BinaryOp::ShrUInt64 => " >>> ",
                    _ => " <OP> ",
                };
                self.output.push_str(op_str);
                self.walk_inline_expression(func, *right);
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
                    self.walk_inline_expression(func, *value);
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
            ExpressionKind::LocalSet { index, value } => {
                let name = self.get_local_name(func, *index, Some(expr));
                self.output.push_str(&format!("({} = ", name));
                self.walk_inline_expression(func, *value);
                self.output.push_str(")");
            }
            ExpressionKind::LocalTee { index, value } => {
                let name = self.get_local_name(func, *index, Some(expr));
                self.output.push_str(&format!("({} = ", name));
                self.walk_inline_expression(func, *value);
                self.output.push_str(")");
            }
            ExpressionKind::GlobalGet { index } => {
                self.output.push_str(&format!("g{}", index));
            }
            ExpressionKind::GlobalSet { index, value } => {
                self.output.push_str(&format!("(g{} = ", index));
                self.walk_inline_expression(func, *value);
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
                self.walk_inline_expression(func, *ptr);
                self.output.push_str(")");
            }
            ExpressionKind::Store { ptr, value, .. } => {
                self.output.push_str("Store(");
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
            ExpressionKind::Break {
                name,
                condition,
                value,
            } => {
                self.output.push_str(&format!("break_to_{}", name));
                if let Some(val) = value {
                    self.output.push_str("(");
                    self.walk_inline_expression(func, *val);
                    self.output.push_str(")");
                }
                if let Some(cond) = condition {
                    self.output.push_str(" if ");
                    self.walk_inline_expression(func, *cond);
                }
            }
            ExpressionKind::Return { value } => {
                if let Some(val) = value {
                    self.output.push_str("return ");
                    self.walk_inline_expression(func, *val);
                } else {
                    self.output.push_str("return");
                }
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                self.output.push_str("(");
                self.walk_inline_expression(func, *condition);
                self.output.push_str(" ? ");
                self.walk_inline_expression(func, *if_true);
                self.output.push_str(" : ");
                self.walk_inline_expression(func, *if_false);
                self.output.push_str(")");
            }
            ExpressionKind::Drop { value } => {
                self.output.push_str("Drop(");
                self.walk_inline_expression(func, *value);
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
