use binaryen_ir::{Module, ExpressionKind, visitor::Visitor, ExprRef, HighLevelType, Annotation};

/// A pass that identifies operations that produce boolean values.
pub struct IdentifyBooleans;

impl IdentifyBooleans {
    pub fn new() -> Self {
        Self
    }

    pub fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut visitor = BooleanVisitor {
            bool_exprs: Vec::new(),
        };
        
        for func in &mut module.functions {
            if let Some(mut body) = func.body {
                visitor.visit(&mut body);
            }
        }
        
        for expr in visitor.bool_exprs {
            module.set_annotation(expr, Annotation::Type(HighLevelType::Bool));
        }
    }
}

struct BooleanVisitor<'a> {
    bool_exprs: Vec<ExprRef<'a>>,
}

impl<'a> Visitor<'a> for BooleanVisitor<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        match &expr.kind {
            ExpressionKind::Binary { op, .. } => {
                if op.is_relational() {
                    self.bool_exprs.push(*expr);
                }
            }
            ExpressionKind::Unary { op, .. } => {
                if op.is_relational() {
                    self.bool_exprs.push(*expr);
                }
            }
            _ => {}
        }
    }
}
