use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Function;
use crate::visitor::ReadOnlyVisitor;
use bumpalo::collections::Vec as BumpVec;
use std::collections::{HashMap, HashSet};

pub type BlockId = u32;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub preds: Vec<BlockId>,
    pub succs: Vec<BlockId>,
    pub contents: Vec<ExprRef<'static>>, // Using static for now as placeholder for 'a, fixed in impl
}

// We need a way to refer to ExprRef with lifetime 'a in the struct.
// But CFG is usually short-lived or tied to module lifetime.
// Let's use 'a.

#[derive(Debug)]
pub struct BasicBlockA<'a> {
    pub id: BlockId,
    pub preds: Vec<BlockId>,
    pub succs: Vec<BlockId>,
    pub contents: Vec<ExprRef<'a>>,
}

pub struct ControlFlowGraph<'a> {
    pub blocks: Vec<BasicBlockA<'a>>,
    pub entry: BlockId,
}

impl<'a> ControlFlowGraph<'a> {
    pub fn build(func: &Function, entry_expr: ExprRef<'a>) -> Self {
        let mut builder = CFGBuilder::new();
        builder.build(entry_expr);
        builder.cfg
    }
}

struct CFGBuilder<'a> {
    cfg: ControlFlowGraph<'a>,
    current_block: BlockId,
}

impl<'a> CFGBuilder<'a> {
    fn new() -> Self {
        let entry_block = BasicBlockA {
            id: 0,
            preds: vec![],
            succs: vec![],
            contents: vec![],
        };

        Self {
            cfg: ControlFlowGraph {
                blocks: vec![entry_block],
                entry: 0,
            },
            current_block: 0,
        }
    }

    fn create_block(&mut self) -> BlockId {
        let id = self.cfg.blocks.len() as u32;
        self.cfg.blocks.push(BasicBlockA {
            id,
            preds: vec![],
            succs: vec![],
            contents: vec![],
        });
        id
    }

    fn add_edge(&mut self, from: BlockId, to: BlockId) {
        if !self.cfg.blocks[from as usize].succs.contains(&to) {
            self.cfg.blocks[from as usize].succs.push(to);
            self.cfg.blocks[to as usize].preds.push(from);
        }
    }

    fn build(&mut self, expr: ExprRef<'a>) {
        // Simple recursive traversal that splits blocks at control flow
        self.visit(expr);
    }

    fn visit(&mut self, expr: ExprRef<'a>) {
        // Add to current block
        self.cfg.blocks[self.current_block as usize]
            .contents
            .push(expr);

        match &expr.kind {
            ExpressionKind::Block { list, .. } => {
                for child in list.iter() {
                    self.visit(*child);
                }
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                // Condition is in current block
                self.visit(*condition);

                let entry_block = self.current_block;
                let then_block = self.create_block();
                let else_block = if if_false.is_some() {
                    Some(self.create_block())
                } else {
                    None
                };
                let join_block = self.create_block();

                // Edge to then
                self.add_edge(entry_block, then_block);

                // Process Then
                self.current_block = then_block;
                self.visit(*if_true);
                self.add_edge(self.current_block, join_block);

                // Process Else
                if let Some(else_b) = else_block {
                    self.add_edge(entry_block, else_b);
                    self.current_block = else_b;
                    if let Some(false_expr) = if_false {
                        self.visit(*false_expr);
                    }
                    self.add_edge(self.current_block, join_block);
                } else {
                    // If no else, edge from entry to join
                    self.add_edge(entry_block, join_block);
                }

                self.current_block = join_block;
            }
            ExpressionKind::Loop { body, .. } => {
                let entry_block = self.current_block;
                let loop_header = self.create_block();
                let loop_exit = self.create_block();

                self.add_edge(entry_block, loop_header);

                self.current_block = loop_header;
                self.visit(*body);

                // Back edge?
                // In Wasm, back edge is explicit via `br` to loop label.
                // But structurally, the end of loop body falls through to exit.
                self.add_edge(self.current_block, loop_exit);

                self.current_block = loop_exit;
            }
            ExpressionKind::Break { .. } => {
                // Terminator.
                // We need to resolve targets.
                // For simplicity in this initial implementation, we don't fully resolve labels.
                // We just mark it as end of block.
                let next_block = self.create_block();
                // No edge to next block (unconditional break).
                // Edge goes to target (not resolved here yet).
                self.current_block = next_block;
            }
            ExpressionKind::Return { value } => {
                if let Some(v) = value {
                    self.visit(*v);
                }
                let next_block = self.create_block();
                self.current_block = next_block;
            }
            // For other nodes, visit children
            _ => {
                // Use a helper or manual match?
                // ReadOnlyVisitor doesn't help much if we change control flow.
                // But for content collection, we need to visit sub-expressions.
                // NOTE: This basic implementation treats most expressions as linear.
                // Call, Binary, etc. are just added to block and children visited.

                // We need to visit children manually to keep order?
                // Or implement a visitor pattern?
                // Let's do a shallow match for children.
                match &expr.kind {
                    ExpressionKind::Binary { left, right, .. } => {
                        self.visit(*left);
                        self.visit(*right);
                    }
                    ExpressionKind::Unary { value, .. } => {
                        self.visit(*value);
                    }
                    ExpressionKind::Call { operands, .. } => {
                        for op in operands.iter() {
                            self.visit(*op);
                        }
                    }
                    ExpressionKind::LocalSet { value, .. } => self.visit(*value),
                    ExpressionKind::LocalTee { value, .. } => self.visit(*value),
                    ExpressionKind::Drop { value } => self.visit(*value),
                    // ... and others
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind, IrBuilder};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_cfg_construction() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (block
        //   (if (i32.const 1)
        //     (nop)
        //     (nop)
        //   )
        // )

        let const1 = builder.const_(Literal::I32(1));
        let nop1 = builder.nop();
        let nop2 = builder.nop();
        let if_ = builder.if_(const1, nop1, Some(nop2), Type::NONE);

        let mut list = BumpVec::new_in(&bump);
        list.push(if_);
        let block = builder.block(None, list, Type::NONE);

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(block),
        );

        let cfg = ControlFlowGraph::build(&func, block);

        // Expected blocks:
        // 0: Entry (block start, if condition) -> 1 (Then), 2 (Else)
        // 1: Then (nop) -> 3 (Join)
        // 2: Else (nop) -> 3 (Join)
        // 3: Join -> Exit

        // Count blocks. We have:
        // Entry block created at start (0)
        // Inside 'if':
        //   Then block created (1)
        //   Else block created (2)
        //   Join block created (3)

        // So total 4 blocks.

        assert_eq!(cfg.blocks.len(), 4);

        // Check edges
        // 0 -> 1
        assert!(cfg.blocks[0].succs.contains(&1));
        // 0 -> 2
        assert!(cfg.blocks[0].succs.contains(&2));

        // 1 -> 3
        assert!(cfg.blocks[1].succs.contains(&3));

        // 2 -> 3
        assert!(cfg.blocks[2].succs.contains(&3));
    }
}
