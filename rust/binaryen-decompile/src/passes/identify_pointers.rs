use binaryen_ir::{visitor::Visitor, Annotation, ExprRef, ExpressionKind, HighLevelType, Module};

/// A pass that identifies expressions that are likely memory pointers.
pub struct IdentifyPointers;

impl IdentifyPointers {
    pub fn new() -> Self {
        Self
    }

    pub fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut pointers = Vec::new();

        for func in &module.functions {
            if let Some(body) = func.body {
                self.find_pointer_sinks(body, &mut pointers);
            }
        }

        // Propagate pointers backwards
        for expr in pointers {
            self.propagate_pointer(expr, module);
        }
    }

    fn find_pointer_sinks<'a>(&self, expr: ExprRef<'a>, pointers: &mut Vec<ExprRef<'a>>) {
        match &expr.kind {
            ExpressionKind::Load { ptr, .. } | ExpressionKind::Store { ptr, .. } => {
                pointers.push(*ptr);
            }
            _ => {}
        }

        // Generic recursion
        match &expr.kind {
            ExpressionKind::Block { list, .. } => {
                for &child in list {
                    self.find_pointer_sinks(child, pointers);
                }
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.find_pointer_sinks(*condition, pointers);
                self.find_pointer_sinks(*if_true, pointers);
                if let Some(f) = if_false {
                    self.find_pointer_sinks(*f, pointers);
                }
            }
            ExpressionKind::Loop { body, .. } => {
                self.find_pointer_sinks(*body, pointers);
            }
            ExpressionKind::Binary { left, right, .. } => {
                self.find_pointer_sinks(*left, pointers);
                self.find_pointer_sinks(*right, pointers);
            }
            ExpressionKind::Unary { value, .. } => {
                self.find_pointer_sinks(*value, pointers);
            }
            ExpressionKind::LocalSet { value, .. } | ExpressionKind::LocalTee { value, .. } => {
                self.find_pointer_sinks(*value, pointers);
            }
            ExpressionKind::Drop { value } => {
                self.find_pointer_sinks(*value, pointers);
            }
            _ => {}
        }
    }

    fn propagate_pointer<'a>(&self, expr: ExprRef<'a>, module: &mut Module<'a>) {
        if let Some(Annotation::Type(HighLevelType::Pointer)) = module.get_annotation(expr) {
            return;
        }

        module.set_annotation(expr, Annotation::Type(HighLevelType::Pointer));

        match &expr.kind {
            ExpressionKind::Binary { op, left, right } => {
                use binaryen_ir::BinaryOp;
                if *op == BinaryOp::AddInt32 || *op == BinaryOp::SubInt32 {
                    // In pointer arithmetic, usually the first operand is the pointer
                    self.propagate_pointer(*left, module);
                }
            }
            _ => {}
        }
    }
}
