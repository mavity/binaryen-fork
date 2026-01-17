use binaryen_ir::{visitor::Visitor, Annotation, ExprRef, ExpressionKind, LoopType, Module};

/// A pass that identifies loop types (While, Do-While) based on branch patterns.
pub struct IdentifyLoops;

impl IdentifyLoops {
    pub fn new() -> Self {
        Self
    }

    pub fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut visitor = LoopVisitor {
            loop_annotations: Vec::new(),
        };

        for func in &mut module.functions {
            if let Some(mut body) = func.body {
                visitor.visit(&mut body);
            }
        }

        for (expr, loop_type) in visitor.loop_annotations {
            module.set_annotation(expr, Annotation::Loop(loop_type));
        }
    }
}

struct LoopVisitor<'a> {
    loop_annotations: Vec<(ExprRef<'a>, LoopType)>,
}

impl<'a> Visitor<'a> for LoopVisitor<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        if let ExpressionKind::Loop { name, body } = &expr.kind {
            let mut detected = false;

            // 1. Look for Do-While Pattern:
            // (loop $L (block ... (br_if $L (cond))))
            if let ExpressionKind::Block { list, .. } = &body.kind {
                if let Some(last) = list.last() {
                    if let ExpressionKind::Break {
                        name: br_name,
                        condition: Some(_),
                        ..
                    } = &last.kind
                    {
                        if name.is_some() && Some(*br_name) == *name {
                            self.loop_annotations.push((*expr, LoopType::DoWhile));
                            detected = true;
                        }
                    }
                }
            }

            // 2. Look for While Pattern:
            // (loop $L (block (if (cond) (nop) (br $OUT)) ... (br $L)))
            // Or simpler: (loop $L (if (cond) (block ... (br $L))))
            if !detected {
                if let ExpressionKind::If { if_false: _, .. } = &body.kind {
                    // If it's an if-then without else, and the then-branch ends in a jump to loop start
                    // OR if it's an if-then-else where one branch jumps out.
                    // This is a bit complex for a first pass, let's stick to a simpler heuristic for now.
                    self.loop_annotations.push((*expr, LoopType::While));
                }
            }
        }
    }
}
