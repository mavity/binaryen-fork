use binaryen_ir::analysis::cfg::ControlFlowGraph;
use binaryen_ir::annotation::Annotation;
use binaryen_ir::{ExpressionKind, Module};

pub struct IdentifyIfElse;

impl IdentifyIfElse {
    pub fn new() -> Self {
        Self
    }

    pub fn run<'a>(&self, module: &mut Module<'a>) {
        let mut annotations = std::collections::HashMap::new();

        for func in module.functions.iter() {
            if let Some(body) = func.body {
                let cfg = ControlFlowGraph::build(func, body);
                self.visit_expression(body, &cfg, &mut annotations);
            }
        }

        // Apply annotations
        for (expr, ann) in annotations {
            module.set_annotation(expr, ann);
        }
    }

    fn visit_expression<'a>(
        &self,
        expr: binaryen_ir::ExprRef<'a>,
        cfg: &ControlFlowGraph<'a>,
        annotations: &mut binaryen_ir::annotation::AnnotationStore<'a>,
    ) {
        match &expr.kind {
            ExpressionKind::Block { name, list, .. } => {
                if let Some(label) = name {
                    if !list.is_empty() {
                        if let ExpressionKind::Break {
                            name: target,
                            condition: Some(cond),
                            ..
                        } = &list[0].kind
                        {
                            if target == label {
                                // Double check with CFG: Does this br_if target the end of this block?
                                if let Some(&block_id) = cfg.expr_to_block.get(&list[0]) {
                                    // In our CFG, a unconditional break to block end
                                    // should have an edge to the join block.
                                    // But since it's conditional, it has 2 succs.
                                    let bb = &cfg.blocks[block_id as usize];
                                    if bb.succs.len() == 2 {
                                        annotations.insert(
                                            expr,
                                            Annotation::If {
                                                condition: *cond,
                                                inverted: true,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                for child in list.iter() {
                    self.visit_expression(*child, cfg, annotations);
                }
            }
            _ => {
                // Visit sub-expressions
                match &expr.kind {
                    ExpressionKind::If {
                        if_true, if_false, ..
                    } => {
                        self.visit_expression(*if_true, cfg, annotations);
                        if let Some(f) = if_false {
                            self.visit_expression(*f, cfg, annotations);
                        }
                    }
                    ExpressionKind::Loop { body, .. } => {
                        self.visit_expression(*body, cfg, annotations);
                    }
                    _ => {}
                }
            }
        }
    }
}
