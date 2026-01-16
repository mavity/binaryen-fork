use crate::analysis::cfg::{BlockId, ControlFlowGraph};
use crate::analysis::dominators::DominanceTree;
use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Function;
use bumpalo::Bump;
use std::collections::{HashMap, HashSet};

pub type LocalId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefA<'a> {
    Instruction(ExprRef<'a>),
    Phi(BlockId, LocalId),
    Param(u32),
}

#[derive(Debug)]
pub struct PhiNode<'a> {
    pub result_local: LocalId,
    // (Incoming Block, Value)
    // Value might be ExprRef if we resolved it, or just placeholder?
    // For SSA construction, we usually need the version of the local in that block.
    // Since we don't have full renaming yet, let's just track which block it comes from.
    pub incoming: Vec<BlockId>,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

pub struct SSABuilder<'a> {
    pub phi_nodes: HashMap<BlockId, Vec<PhiNode<'a>>>,
    // Map from original local to its SSA versions (just count/ids for now?)
    // Or map LocalId -> Set of blocks where it is defined (including Phis)
    pub def_blocks: HashMap<LocalId, HashSet<BlockId>>,
    pub use_def: HashMap<ExprRef<'a>, DefA<'a>>,
}

impl<'a> SSABuilder<'a> {
    pub fn new() -> Self {
        Self {
            phi_nodes: HashMap::new(),
            def_blocks: HashMap::new(),
            use_def: HashMap::new(),
        }
    }

    pub fn build(func: &Function, cfg: &ControlFlowGraph<'a>, dom: &DominanceTree) -> Self {
        let mut builder = Self::new();
        builder.compute_phi_placements(cfg, dom);
        builder.rename_variables(func, cfg, dom);
        builder
    }

    fn rename_variables(
        &mut self,
        _func: &Function,
        cfg: &ControlFlowGraph<'a>,
        dom: &DominanceTree,
    ) {
        let mut stacks: HashMap<LocalId, Vec<DefA<'a>>> = HashMap::new();
        self.rename_block(cfg.entry, cfg, dom, &mut stacks);
    }

    fn rename_block(
        &mut self,
        block_id: BlockId,
        cfg: &ControlFlowGraph<'a>,
        dom: &DominanceTree,
        stacks: &mut HashMap<LocalId, Vec<DefA<'a>>>,
    ) {
        let mut pushes: HashMap<LocalId, usize> = HashMap::new();

        // Phis
        if let Some(phis) = self.phi_nodes.get(&block_id) {
            for phi in phis {
                let def = DefA::Phi(block_id, phi.result_local);
                stacks.entry(phi.result_local).or_default().push(def);
                *pushes.entry(phi.result_local).or_default() += 1;
            }
        }

        // Instructions
        if let Some(block) = cfg.blocks.get(block_id as usize) {
            for expr in &block.contents {
                self.visit_expr_for_rename(*expr, stacks, &mut pushes);
            }

            // Recurse children
            // Naive search for children
            // In efficient impl, dom tree stores children list.
            for (child, &parent) in &dom.doms {
                if parent == block_id && *child != block_id {
                    self.rename_block(*child, cfg, dom, stacks);
                }
            }
        }

        // Pop
        for (local, count) in pushes {
            let stack = stacks.get_mut(&local).unwrap();
            for _ in 0..count {
                stack.pop();
            }
        }
    }

    fn visit_expr_for_rename(
        &mut self,
        expr: ExprRef<'a>,
        stacks: &mut HashMap<LocalId, Vec<DefA<'a>>>,
        pushes: &mut HashMap<LocalId, usize>,
    ) {
        match &expr.kind {
            ExpressionKind::LocalGet { index } => {
                if let Some(stack) = stacks.get(index) {
                    if let Some(def) = stack.last() {
                        self.use_def.insert(expr, *def);
                    }
                }
            }
            ExpressionKind::LocalSet { index, value }
            | ExpressionKind::LocalTee { index, value } => {
                self.visit_expr_for_rename(*value, stacks, pushes);
                let def = DefA::Instruction(expr);
                stacks.entry(*index).or_default().push(def);
                *pushes.entry(*index).or_default() += 1;
            }
            ExpressionKind::Block { list, .. } => {
                for child in list.iter() {
                    self.visit_expr_for_rename(*child, stacks, pushes);
                }
            }
            // Add other traversals as needed...
            _ => {}
        }
    }

    fn compute_phi_placements(&mut self, cfg: &ControlFlowGraph<'a>, dom: &DominanceTree) {
        // 1. Compute join points for each local (where Phis are needed)
        // Set of blocks that define each local
        let mut defs: HashMap<LocalId, HashSet<BlockId>> = HashMap::new();

        // Scan CFG to find defs
        for block in &cfg.blocks {
            for expr in &block.contents {
                match &expr.kind {
                    ExpressionKind::LocalSet { index, .. }
                    | ExpressionKind::LocalTee { index, .. } => {
                        defs.entry(*index).or_default().insert(block.id);
                    }
                    // Params are defs in entry block?
                    // Usually params are considered defined in entry.
                    // We need to know which indices are params.
                    // func.params provides types but not indices directly (implicit 0..N).
                    _ => {}
                }
            }
        }

        // Add params as defs in entry block (0)
        // func.params is Type (if tuple, it has multiple components)
        // For simplicity, let's assume we scan func.params or vars count?
        // We iterate found locals.
        // Actually, if a local is used, it must be defined.
        // If it is a param, it is defined in entry.
        // We can check func.params count?
        // `Type` doesn't expose count easily without `expand`.
        // Let's assume locals 0..params_count are defined in entry.
        // For now, we only rely on explicit Sets found in body.

        // 2. Insert Phi nodes at dominance frontiers
        for (local, blocks) in &defs {
            let mut worklist: Vec<BlockId> = blocks.iter().cloned().collect();
            let mut visited = HashSet::new();

            // Also add blocks already in visited to avoid reprocessing?
            // Actually `visited` tracks where we inserted Phi.
            // `has_already` tracks if we added block to worklist?
            // Standard algorithm uses `has_phi` and `has_def`.

            while let Some(b) = worklist.pop() {
                if let Some(frontier) = dom.frontiers.get(&b) {
                    for &f in frontier {
                        if !visited.contains(&f) {
                            visited.insert(f);

                            // Insert Phi for `local` at block `f`
                            self.phi_nodes.entry(f).or_default().push(PhiNode {
                                result_local: *local,
                                incoming: vec![], // Populated later in renaming
                                _marker: std::marker::PhantomData,
                            });

                            // Phi node is a new def, so add to worklist
                            if !blocks.contains(&f) {
                                worklist.push(f);
                            }
                        }
                    }
                }
            }
        }

        self.def_blocks = defs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::cfg::ControlFlowGraph;
    use crate::analysis::dominators::DominanceTree;
    use crate::expression::{Expression, ExpressionKind, IrBuilder};
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_ssa_phi_placement() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (block
        //   (if (i32.const 1)
        //     (local.set 0 (i32.const 10))
        //     (local.set 0 (i32.const 20))
        //   )
        //   (local.get 0)
        // )

        // Join block (block 3 in CFG) should have Phi for local 0.

        let const1 = builder.const_(Literal::I32(1));
        let const10 = builder.const_(Literal::I32(10));
        let const20 = builder.const_(Literal::I32(20));

        let set0_1 = builder.local_set(0, const10);
        let set0_2 = builder.local_set(0, const20);

        let if_ = builder.if_(const1, set0_1, Some(set0_2), Type::NONE);
        let get0 = builder.local_get(0, Type::I32);

        let mut list = bumpalo::collections::Vec::new_in(&bump);
        list.push(if_);
        list.push(get0);

        let block = builder.block(None, list, Type::NONE);

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32],
            Some(block),
        );

        let cfg = ControlFlowGraph::build(&func, block);
        let dom = DominanceTree::build(&cfg);

        let ssa = SSABuilder::build(&func, &cfg, &dom);

        // CFG structure:
        // 0: Entry -> 1, 2
        // 1: Then (set 0) -> 3
        // 2: Else (set 0) -> 3
        // 3: Join (get 0) -> Exit

        // Local 0 defined in 1 and 2.
        // DF(1) = {3}
        // DF(2) = {3}
        // So Phi should be in 3.

        let phis = ssa.phi_nodes.get(&3);
        assert!(phis.is_some());
        assert_eq!(phis.unwrap().len(), 1);
        assert_eq!(phis.unwrap()[0].result_local, 0);
    }

    #[test]
    fn test_ssa_renaming() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (block
        //   (local.set 0 (i32.const 10))  <-- Def 1
        //   (local.get 0)                 <-- Use 1 (should point to Def 1)
        //   (local.set 0 (i32.const 20))  <-- Def 2
        //   (local.get 0)                 <-- Use 2 (should point to Def 2)
        // )

        let const10 = builder.const_(Literal::I32(10));
        let set1 = builder.local_set(0, const10);

        let get1 = builder.local_get(0, Type::I32);

        let const20 = builder.const_(Literal::I32(20));
        let set2 = builder.local_set(0, const20);

        let get2 = builder.local_get(0, Type::I32);

        let mut list = bumpalo::collections::Vec::new_in(&bump);
        list.push(set1);
        list.push(get1);
        list.push(set2);
        list.push(get2);

        let block = builder.block(None, list, Type::NONE);
        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32],
            Some(block),
        );

        let cfg = ControlFlowGraph::build(&func, block);
        let dom = DominanceTree::build(&cfg);
        let ssa = SSABuilder::build(&func, &cfg, &dom);

        // Check Use 1 -> Def 1
        if let Some(DefA::Instruction(def)) = ssa.use_def.get(&get1) {
            assert_eq!(def.as_ptr(), set1.as_ptr());
        } else {
            panic!("Use 1 not found or wrong type");
        }

        // Check Use 2 -> Def 2
        if let Some(DefA::Instruction(def)) = ssa.use_def.get(&get2) {
            assert_eq!(def.as_ptr(), set2.as_ptr());
        } else {
            panic!("Use 2 not found or wrong type");
        }
    }

    #[test]
    fn test_ssa_simple_linear() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (block
        //   (local.set 0 (i32.const 1))
        //   (local.get 0)
        // )

        let c1 = builder.const_(Literal::I32(1));
        let set1 = builder.local_set(0, c1);
        let get1 = builder.local_get(0, Type::I32);

        let mut list = bumpalo::collections::Vec::new_in(&bump);
        list.push(set1);
        list.push(get1);
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
        let ssa = SSABuilder::build(&func, &cfg, &dom);

        // Should find use-def
        if let Some(DefA::Instruction(def)) = ssa.use_def.get(&get1) {
            assert_eq!(def.as_ptr(), set1.as_ptr());
        } else {
            panic!("Use not resolved");
        }

        // No Phis needed
        assert!(ssa.phi_nodes.is_empty());
    }
}
