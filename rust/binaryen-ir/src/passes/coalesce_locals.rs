use crate::dataflow::cfg::CFGBuilder;
use crate::dataflow::liveness::InterferenceGraph;
use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Function;
use crate::visitor::Visitor;
use binaryen_core::Type;

pub struct CoalesceLocals;

struct LocalMapper<'a> {
    mapping: &'a [u32],
}

impl<'a, 'b> Visitor<'a> for LocalMapper<'b> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        if let Some(expr_mut) = unsafe { expr.as_ptr().as_mut() } {
            match &mut expr_mut.kind {
                ExpressionKind::LocalGet { index } => {
                    if let Some(&new_idx) = self.mapping.get(*index as usize) {
                        *index = new_idx;
                    }
                }
                ExpressionKind::LocalSet { index, .. } => {
                    if let Some(&new_idx) = self.mapping.get(*index as usize) {
                        *index = new_idx;
                    }
                }
                ExpressionKind::LocalTee { index, .. } => {
                    if let Some(&new_idx) = self.mapping.get(*index as usize) {
                        *index = new_idx;
                    }
                }
                _ => {}
            }
        }
    }
}

impl CoalesceLocals {
    pub fn run(func: &mut Function) {
        if let Some(body) = &mut func.body {
            // 1. Build CFG & Liveness
            // CFGBuilder takes &'a mut Expression<'a>
            // We need to verify lifetimes here.
            // func.body is Option<ExprRef<'a>>.
            // ExprRef<'a> points to Expression<'a>.
            // CFGBuilder requires mutable reference to Expression.

            // To mutate via ExprRef, we usually deref_mut.
            // ExprRef impl DerefMut.

            let root = &mut *body;

            let mut builder = CFGBuilder::new();
            let mut cfg = builder.build(root);
            cfg.calculate_liveness();

            // 2. Interference
            let interference = cfg.calculate_interference();

            // 3. Coloring
            let num_params = Self::count_types(func.params);

            // Construct full type list
            let mut types = Vec::new();
            Self::append_types(&mut types, func.params);
            types.extend_from_slice(&func.vars);

            let mapping = Self::color(num_params as u32, &types, &interference);

            // 4. Update Function vars
            let mut new_vars = Vec::new();
            // We need to reconstruct new_vars.
            // We iterate mapping.
            // If mapping[i] >= num_params, it's a new var.
            // index = mapping[i] - num_params.
            // resize new_vars if needed.

            for (old_idx, &new_idx) in mapping.iter().enumerate() {
                if new_idx >= num_params as u32 {
                    let internal_idx = (new_idx - num_params as u32) as usize;
                    if internal_idx >= new_vars.len() {
                        new_vars.resize(internal_idx + 1, Type::NONE);
                    }
                    if new_vars[internal_idx] == Type::NONE {
                        new_vars[internal_idx] = types[old_idx];
                    }
                }
            }
            func.vars = new_vars;

            // 5. Update body
            // We need a separate pass to update the body AST since CFGBuilder consumed it?
            // No, CFGBuilder visit mutable ref, it doesn't consume ExprRef ownership (it takes &mut).
            // But we can't use `cfg` anymore if it borrowed from `root`?
            // `cfg` has lifetime 'a. `root` has lifetime 'a.
            // But `mapping` is just Vec<u32>. `interference` is owned (return from calculate_interference).
            // `cfg` needs to be dropped or not conflict.
            // `calculate_interference` takes `&mut cfg`.
            // Once we have `interference`, we can drop `cfg`.

            drop(cfg); // Release borrow on root?
                       // Actually CFGBuilder::build took `&'a mut Expression`.
                       // The borrow lasts as long as `cfg` exists?
                       // CFGBuilder definition:
                       // struct CFGBuilder<'a> { ... _marker: PhantomData<&'a mut ...> }
                       // ControlFlowGraph<'a> { ... actions: Vec<LivenessAction<'a>> }
                       // LivenessAction<'a> holds ExprRef<'a>.
                       // ExprRef<'a> is Copy/Clone pointer. It doesn't borrow logic-wise in Rust type system (it's a raw ptr wrapper).
                       // It assumes the Arena lives for 'a.
                       // So dropping `cfg` is fine/irrelevant to mutable borrow of `root`?
                       // Wait, I passed `&mut *body` to `builder.build`.
                       // `builder.build(root)` takes `&'a mut Expression`.
                       // But `ExprRef::new` creates a pointer.
                       // If `CFGBuilder` doesn't hold the `&mut`, we are fine.
                       // My implementation of `visit` created `ExprRef` from pointers.
                       // So `CFGBuilder` builds a structure holding `ExprRef`.
                       // `ExprRef` is `Copy`.

            let mut mapper = LocalMapper { mapping: &mapping };
            mapper.visit(body);
        }
    }

    fn count_types(ty: Type) -> usize {
        if ty == Type::NONE {
            0
        }
        // TODO: Handle tuples
        else {
            1
        }
    }

    fn append_types(list: &mut Vec<Type>, ty: Type) {
        if ty == Type::NONE {
            return;
        }
        // TODO: Handle tuples
        list.push(ty);
    }

    fn color(num_params: u32, types: &[Type], graph: &InterferenceGraph) -> Vec<u32> {
        let num_locals = types.len() as u32;
        let mut mapping: Vec<u32> = (0..num_locals).collect();
        // new_vars[j] stores list of old locals assigned to new var j (relative to params)
        let mut new_vars: Vec<Vec<u32>> = Vec::new();

        for i in num_params..num_locals {
            let mut found = false;
            for (j, assigned) in new_vars.iter_mut().enumerate() {
                // Check type compatibility
                if assigned.is_empty() {
                    continue;
                } // Should not happen
                let type_j = types[assigned[0] as usize];
                if types[i as usize] != type_j {
                    continue;
                }

                // Check interference
                let mut interferes = false;
                for &other in assigned.iter() {
                    if graph.interferes(i, other) {
                        interferes = true;
                        break;
                    }
                }

                if !interferes {
                    mapping[i as usize] = num_params + j as u32;
                    assigned.push(i);
                    found = true;
                    break;
                }
            }

            if !found {
                mapping[i as usize] = num_params + new_vars.len() as u32;
                new_vars.push(vec![i]);
            }
        }

        mapping
    }
}
