use crate::analysis::cfg::{BlockId, ControlFlowGraph};
use std::collections::{HashMap, HashSet};

pub struct DominanceTree {
    pub doms: HashMap<BlockId, BlockId>, // id -> immediate dominator
    pub frontiers: HashMap<BlockId, HashSet<BlockId>>,
}

impl DominanceTree {
    pub fn build(cfg: &ControlFlowGraph) -> Self {
        let mut doms = HashMap::new();
        let blocks = &cfg.blocks;
        let num_blocks = blocks.len();

        if num_blocks == 0 {
            return Self {
                doms,
                frontiers: HashMap::new(),
            };
        }

        // Initialize dominators
        // Entry dominates itself
        doms.insert(cfg.entry, cfg.entry);

        // Others initialized to all blocks (represented as None or specific check)
        // Here we use iterative algorithm

        // Initial state: dom(entry) = {entry}, others = all
        let mut all_nodes: HashSet<BlockId> = blocks.iter().map(|b| b.id).collect();
        let mut dom_sets: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

        dom_sets.insert(cfg.entry, [cfg.entry].iter().cloned().collect());
        for b in blocks {
            if b.id != cfg.entry {
                dom_sets.insert(b.id, all_nodes.clone());
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for b in blocks {
                if b.id == cfg.entry {
                    continue;
                }

                // dom(n) = {n} U (intersection of dom(p) for all preds p)
                let mut new_dom = all_nodes.clone(); // Start with all

                if b.preds.is_empty() {
                    // Unreachable from entry?
                    // Should be handled, but intersect of empty is empty (or universe?)
                    // In data flow, intersection over empty set is usually universe (Top).
                    // But if it has no preds and is not entry, it's unreachable.
                    // We skip unreachable blocks in dominance calculation usually.
                    continue;
                }

                let mut first = true;
                for &pred in &b.preds {
                    if let Some(pred_doms) = dom_sets.get(&pred) {
                        if first {
                            new_dom = pred_doms.clone();
                            first = false;
                        } else {
                            new_dom.retain(|x| pred_doms.contains(x));
                        }
                    }
                }

                new_dom.insert(b.id);

                if dom_sets.get(&b.id) != Some(&new_dom) {
                    dom_sets.insert(b.id, new_dom);
                    changed = true;
                }
            }
        }

        // Compute immediate dominators
        for (&n, dominators) in &dom_sets {
            // idom(n) is the unique node d in dominators such that d != n and d dominates every other node in dominators\{n}
            if n == cfg.entry {
                continue;
            }

            let mut candidates: Vec<BlockId> =
                dominators.iter().cloned().filter(|&d| d != n).collect();
            // Find the one that is dominated by all others (closest)
            // Actually, idom dominates n, and is dominated by all strict dominators of n.
            // So idom is the "largest" strict dominator.
            // A dominates B if A is in dom_sets[B].

            if let Some(&idom) = candidates.iter().find(|&&candidate| {
                // Check if candidate is dominated by all other candidates
                candidates.iter().all(|&other| {
                    if other == candidate {
                        true
                    } else {
                        // other dominates candidate?
                        dom_sets.get(&candidate).unwrap().contains(&other)
                    }
                })
            }) {
                doms.insert(n, idom);
            }
        }

        // Compute dominance frontiers
        // DF(n) = { y | n dom pred(y) but not n sdom y }
        let mut frontiers = HashMap::new();

        for b in blocks {
            if b.preds.len() >= 2 {
                for &p in &b.preds {
                    let mut runner = p;
                    while runner != *doms.get(&b.id).unwrap_or(&cfg.entry) {
                        // Approx check
                        // Add b to frontier of runner
                        frontiers
                            .entry(runner)
                            .or_insert_with(HashSet::new)
                            .insert(b.id);

                        if let Some(&d) = doms.get(&runner) {
                            if d == runner {
                                break;
                            } // Root
                            runner = d;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Self { doms, frontiers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind, IrBuilder};
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_dominance_diamond() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Diamond shape CFG (like if-then-else)
        // 0 -> 1, 2
        // 1 -> 3
        // 2 -> 3

        let const1 = builder.const_(Literal::I32(1));
        let nop1 = builder.nop();
        let nop2 = builder.nop();
        let if_ = builder.if_(const1, nop1, Some(nop2), Type::NONE);

        let mut list = bumpalo::collections::Vec::new_in(&bump);
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
        let dom = DominanceTree::build(&cfg);

        // Dom tree should be:
        // 0 dominates 1, 2, 3
        // 1 does not dominate 3 (because path through 2 exists)
        // 2 does not dominate 3

        assert_eq!(*dom.doms.get(&0).unwrap(), 0);
        assert_eq!(*dom.doms.get(&1).unwrap(), 0);
        assert_eq!(*dom.doms.get(&2).unwrap(), 0);
        assert_eq!(*dom.doms.get(&3).unwrap(), 0);

        // Frontiers:
        // DF(1) = {3} (1 dominates predecessor of 3, but not 3)
        // DF(2) = {3}

        let empty = HashSet::new();
        let df1 = dom.frontiers.get(&1).unwrap_or(&empty);
        assert!(df1.contains(&3));

        let df2 = dom.frontiers.get(&2).unwrap_or(&empty);
        assert!(df2.contains(&3));
    }
}
