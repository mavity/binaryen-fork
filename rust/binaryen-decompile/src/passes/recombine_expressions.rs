use binaryen_ir::annotation::Annotation;
use binaryen_ir::expression::{ExprRef, ExpressionKind};
use binaryen_ir::module::Module;
use binaryen_ir::visitor::Visitor;
use std::collections::HashMap;

pub struct ExpressionRecombination;

impl ExpressionRecombination {
    pub fn run<'a>(module: &mut Module<'a>) {
        for i in 0..module.functions.len() {
            let func = &module.functions[i];
            if let Some(mut body) = func.body {
                let mut analyzer = UsageAnalyzer::new();
                analyzer.visit(&mut body);

                let mut inliner = Inliner { module, analyzer };
                inliner.process(body);
            }
        }
    }
}

struct UsageAnalyzer<'a> {
    set_counts: HashMap<u32, usize>,
    get_counts: HashMap<u32, usize>,
    last_set: HashMap<u32, ExprRef<'a>>,
}

impl<'a> UsageAnalyzer<'a> {
    fn new() -> Self {
        Self {
            set_counts: HashMap::new(),
            get_counts: HashMap::new(),
            last_set: HashMap::new(),
        }
    }
}

impl<'a> Visitor<'a> for UsageAnalyzer<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        match &expr.kind {
            ExpressionKind::LocalSet { index, .. } => {
                *self.set_counts.entry(*index).or_insert(0) += 1;
                self.last_set.insert(*index, *expr);
            }
            ExpressionKind::LocalGet { index } => {
                *self.get_counts.entry(*index).or_insert(0) += 1;
            }
            _ => {}
        }
    }
}

struct Inliner<'a, 'b> {
    module: &'b mut Module<'a>,
    analyzer: UsageAnalyzer<'a>,
}

impl<'a, 'b> Inliner<'a, 'b> {
    fn process(&mut self, expr: ExprRef<'a>) {
        self.annotate(expr);

        match &expr.kind {
            ExpressionKind::Block { list, .. } => {
                for &child in list {
                    self.process(child);
                }
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.process(*condition);
                self.process(*if_true);
                if let Some(f) = if_false {
                    self.process(*f);
                }
            }
            ExpressionKind::Loop { body, .. } => {
                self.process(*body);
            }
            ExpressionKind::LocalSet { value, .. } | ExpressionKind::LocalTee { value, .. } => {
                self.process(*value);
            }
            ExpressionKind::Binary { left, right, .. } => {
                self.process(*left);
                self.process(*right);
            }
            ExpressionKind::Unary { value, .. } => {
                self.process(*value);
            }
            ExpressionKind::Call { operands, .. } => {
                for &op in operands {
                    self.process(op);
                }
            }
            ExpressionKind::Load { ptr, .. } => {
                self.process(*ptr);
            }
            ExpressionKind::Store { ptr, value, .. } => {
                self.process(*ptr);
                self.process(*value);
            }
            ExpressionKind::Drop { value } => {
                self.process(*value);
            }
            _ => {}
        }
    }

    fn annotate(&mut self, expr: ExprRef<'a>) {
        match &expr.kind {
            ExpressionKind::LocalSet { index, value } => {
                if self.analyzer.set_counts.get(index) == Some(&1)
                    && self.analyzer.get_counts.get(index) == Some(&1)
                {
                    if is_simple_expression(*value) {
                        self.module.set_annotation(expr, Annotation::Inlined);
                    }
                }
            }
            ExpressionKind::LocalGet { index } => {
                if self.analyzer.set_counts.get(index) == Some(&1)
                    && self.analyzer.get_counts.get(index) == Some(&1)
                {
                    if let Some(set_expr) = self.analyzer.last_set.get(index) {
                        if let ExpressionKind::LocalSet { value, .. } = &set_expr.kind {
                            if is_simple_expression(*value) {
                                self.module
                                    .set_annotation(expr, Annotation::InlinedValue(*value));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn is_simple_expression(expr: ExprRef) -> bool {
    match &expr.kind {
        ExpressionKind::Const(_) => true,
        ExpressionKind::LocalGet { .. } => true,
        ExpressionKind::Binary { left, right, .. } => {
            is_simple_expression(*left) && is_simple_expression(*right)
        }
        ExpressionKind::Unary { value, .. } => is_simple_expression(*value),
        _ => false, // Load, Call, etc. are NOT simple (they have side effects or depend on state)
    }
}
