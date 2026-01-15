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
            let root = &mut *body;

            let builder = CFGBuilder::new();
            let mut cfg = builder.build(root);
            cfg.calculate_liveness();

            let interference = cfg.calculate_interference();

            let num_params = Self::count_types(func.params);

            let mut types = Vec::new();
            Self::append_types(&mut types, func.params);
            types.extend_from_slice(&func.vars);

            let mapping = Self::color(num_params as u32, &types, &interference);

            let mut new_vars = Vec::new();
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

            drop(cfg);

            let mut mapper = LocalMapper { mapping: &mapping };
            mapper.visit(body);
        }
    }

    fn count_types(ty: Type) -> usize {
        if ty == Type::NONE {
            0
        } else {
            1
        }
    }

    fn append_types(list: &mut Vec<Type>, ty: Type) {
        if ty == Type::NONE {
            return;
        }
        list.push(ty);
    }

    fn color(num_params: u32, types: &[Type], graph: &InterferenceGraph) -> Vec<u32> {
        let num_locals = types.len() as u32;
        let mut mapping: Vec<u32> = (0..num_locals).collect();
        let mut new_vars: Vec<Vec<u32>> = Vec::new();

        for i in num_params..num_locals {
            let mut found = false;
            for (j, assigned) in new_vars.iter_mut().enumerate() {
                if assigned.is_empty() {
                    continue;
                }
                let type_j = types[assigned[0] as usize];
                if types[i as usize] != type_j {
                    continue;
                }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{ExprRef, Expression, ExpressionKind};
    use binaryen_core::{Literal, Type};
    use bumpalo::collections::Vec as BumpVec;
    use bumpalo::Bump;

    fn alloc_expr<'a>(bump: &'a Bump, kind: ExpressionKind<'a>, ty: Type) -> ExprRef<'a> {
        let expr = bump.alloc(Expression { type_: ty, kind });
        ExprRef::new(expr)
    }

    #[test]
    fn test_merge_disjoint_locals() {
        let bump = Bump::new();

        let c10 = alloc_expr(&bump, ExpressionKind::Const(Literal::I32(10)), Type::I32);
        let set0 = alloc_expr(
            &bump,
            ExpressionKind::LocalSet {
                index: 0,
                value: c10,
            },
            Type::NONE,
        );
        let get0 = alloc_expr(&bump, ExpressionKind::LocalGet { index: 0 }, Type::I32);
        let drop0 = alloc_expr(&bump, ExpressionKind::Drop { value: get0 }, Type::NONE);

        let c20 = alloc_expr(&bump, ExpressionKind::Const(Literal::I32(20)), Type::I32);
        let set1 = alloc_expr(
            &bump,
            ExpressionKind::LocalSet {
                index: 1,
                value: c20,
            },
            Type::NONE,
        );
        let get1 = alloc_expr(&bump, ExpressionKind::LocalGet { index: 1 }, Type::I32);
        let drop1 = alloc_expr(&bump, ExpressionKind::Drop { value: get1 }, Type::NONE);

        let mut list = BumpVec::new_in(&bump);
        list.push(set0);
        list.push(drop0);
        list.push(set1);
        list.push(drop1);

        let body = alloc_expr(
            &bump,
            ExpressionKind::Block { name: None, list },
            Type::NONE,
        );

        let mut func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32, Type::I32],
            Some(body),
        );

        CoalesceLocals::run(&mut func);

        assert_eq!(func.vars.len(), 1);
        assert_eq!(func.vars[0], Type::I32);
    }

    #[test]
    fn test_interfering_locals() {
        let bump = Bump::new();

        let c10 = alloc_expr(&bump, ExpressionKind::Const(Literal::I32(10)), Type::I32);
        let set0 = alloc_expr(
            &bump,
            ExpressionKind::LocalSet {
                index: 0,
                value: c10,
            },
            Type::NONE,
        );

        let c20 = alloc_expr(&bump, ExpressionKind::Const(Literal::I32(20)), Type::I32);
        let set1 = alloc_expr(
            &bump,
            ExpressionKind::LocalSet {
                index: 1,
                value: c20,
            },
            Type::NONE,
        );

        let get0 = alloc_expr(&bump, ExpressionKind::LocalGet { index: 0 }, Type::I32);
        let drop0 = alloc_expr(&bump, ExpressionKind::Drop { value: get0 }, Type::NONE);

        let get1 = alloc_expr(&bump, ExpressionKind::LocalGet { index: 1 }, Type::I32);
        let drop1 = alloc_expr(&bump, ExpressionKind::Drop { value: get1 }, Type::NONE);

        let mut list = BumpVec::new_in(&bump);
        list.push(set0);
        list.push(set1);
        list.push(drop0);
        list.push(drop1);

        let body = alloc_expr(
            &bump,
            ExpressionKind::Block { name: None, list },
            Type::NONE,
        );

        let mut func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32, Type::I32],
            Some(body),
        );

        CoalesceLocals::run(&mut func);

        assert_eq!(func.vars.len(), 2);
    }

    #[test]
    fn test_copy_coalescing() {
        let bump = Bump::new();

        let c10 = alloc_expr(&bump, ExpressionKind::Const(Literal::I32(10)), Type::I32);
        let set0 = alloc_expr(
            &bump,
            ExpressionKind::LocalSet {
                index: 0,
                value: c10,
            },
            Type::NONE,
        );

        let get0 = alloc_expr(&bump, ExpressionKind::LocalGet { index: 0 }, Type::I32);
        let set1 = alloc_expr(
            &bump,
            ExpressionKind::LocalSet {
                index: 1,
                value: get0,
            },
            Type::NONE,
        );

        let get1 = alloc_expr(&bump, ExpressionKind::LocalGet { index: 1 }, Type::I32);
        let drop1 = alloc_expr(&bump, ExpressionKind::Drop { value: get1 }, Type::NONE);

        let mut list = BumpVec::new_in(&bump);
        list.push(set0);
        list.push(set1);
        list.push(drop1);

        let body = alloc_expr(
            &bump,
            ExpressionKind::Block { name: None, list },
            Type::NONE,
        );

        let mut func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32, Type::I32],
            Some(body),
        );

        CoalesceLocals::run(&mut func);

        assert_eq!(func.vars.len(), 1);
    }
}
