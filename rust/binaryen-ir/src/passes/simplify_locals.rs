use crate::effects::{Effect, EffectAnalyzer};
use crate::expression::{ExprRef, Expression, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use std::collections::HashMap;

/// SimplifyLocals pass - implements local-related optimizations
///
/// This pass performs "sinking" optimizations for local.set operations:
/// - Pushes local.set operations closer to their local.get usage
/// - Removes local.set operations if no gets remain
/// - Creates local.tee when a local has multiple uses
/// - Merges local.sets into block/if return values
///
/// Options:
/// - allow_tee: Allow creating local.tee for multiple uses
/// - allow_structure: Create block/if return values by merging internal sets
/// - allow_nesting: Allow sinking that creates nested expressions
pub struct SimplifyLocals {
    allow_tee: bool,
    allow_structure: bool,
    allow_nesting: bool,
    another_cycle: bool,
}

impl SimplifyLocals {
    /// Create with all optimizations enabled
    pub fn new() -> Self {
        Self {
            allow_tee: true,
            allow_structure: true,
            allow_nesting: true,
            another_cycle: false,
        }
    }

    /// Create with custom options
    pub fn with_options(allow_tee: bool, allow_structure: bool, allow_nesting: bool) -> Self {
        Self {
            allow_tee,
            allow_structure,
            allow_nesting,
            another_cycle: false,
        }
    }

    /// Create with "flat" mode - no nesting allowed
    pub fn flat() -> Self {
        Self {
            allow_tee: false,
            allow_structure: false,
            allow_nesting: false,
            another_cycle: false,
        }
    }
}

impl Default for SimplifyLocals {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a sinkable local.set
#[derive(Clone, PartialEq)]
struct SinkableInfo<'a> {
    /// Effect analysis of the set operation
    effects: Effect,
    /// The local.set expression itself
    set: ExprRef<'a>,
}

/// Context for a single function optimization
#[allow(dead_code)]
struct FunctionContext<'a> {
    /// Map from local index to sinkable info
    sinkables: HashMap<u32, SinkableInfo<'a>>,
    /// Count of local.get operations per local
    get_counts: HashMap<u32, usize>,
    /// Whether this is the first optimization cycle
    first_cycle: bool,
    /// Options
    allow_tee: bool,
    allow_structure: bool,
    allow_nesting: bool,
}

impl<'a> FunctionContext<'a> {
    fn new(allow_tee: bool, allow_structure: bool, allow_nesting: bool) -> Self {
        Self {
            sinkables: HashMap::new(),
            get_counts: HashMap::new(),
            first_cycle: true,
            allow_tee,
            allow_structure,
            allow_nesting,
        }
    }

    /// Check if a local.set can be sunk
    fn can_sink(&mut self, set: &Expression) -> bool {
        if let ExpressionKind::LocalSet { index, .. } = &set.kind {
            // If in first cycle or not allowing tees, cannot sink if >1 use
            // (would require creating a tee)
            let use_count = self.get_counts.get(index).copied().unwrap_or(0);
            if use_count > 1 {
                if !self.allow_tee {
                    return false;
                }
                if self.first_cycle {
                    // Deferred to next cycle when we can create a tee
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    /// Check if effects invalidate any sinkables
    fn check_invalidations(&mut self, effects: Effect) {
        self.sinkables.retain(|_, info| {
            // If the new effects invalidate this sinkable, remove it
            !effects.interferes_with(info.effects)
        });
    }
}

impl Pass for SimplifyLocals {
    fn name(&self) -> &str {
        "SimplifyLocals"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut first_cycle = true;
        // Run multiple cycles until no more changes
        loop {
            self.another_cycle = false;

            for func in &mut module.functions {
                if let Some(body) = &mut func.body {
                    let mut ctx = FunctionContext::new(
                        self.allow_tee,
                        self.allow_structure,
                        self.allow_nesting,
                    );
                    ctx.first_cycle = first_cycle;

                    // First pass: count local.get operations
                    ctx.get_counts.clear();
                    count_gets(body, &mut ctx.get_counts);

                    // Second pass: optimize
                    self.optimize_function(body, &mut ctx);
                }
            }

            first_cycle = false;
            if !self.another_cycle {
                break;
            }
        }
    }
}

impl SimplifyLocals {
    fn optimize_function<'a>(&mut self, expr: &mut ExprRef<'a>, ctx: &mut FunctionContext<'a>) {
        // Handle LocalGet
        let get_index = if let ExpressionKind::LocalGet { index } = &expr.kind {
            Some(*index)
        } else {
            None
        };
        if let Some(index) = get_index {
            // Sinking logic
            if let Some(info) = ctx.sinkables.remove(&index) {
                unsafe {
                    let set_kind = &mut (*info.set.as_ptr()).kind;
                    if let ExpressionKind::LocalSet { value, .. } = set_kind {
                        if ctx.first_cycle || ctx.get_counts.get(&index).copied().unwrap_or(0) == 1
                        {
                            // Single use: Replace Get with Value
                            *expr = *value;
                        } else {
                            // Multiple uses: Replace Get with LocalTee
                            // We are reusing the LocalGet expression node memory, changing its kind.
                            // The value comes from the LocalSet, which we are about to NOP.
                            expr.kind = ExpressionKind::LocalTee {
                                index,
                                value: *value,
                            };
                            // expr.type_ remains the same (LocalGet type == LocalTee type == Value type)
                        }

                        // Nop out the Set
                        *set_kind = ExpressionKind::Nop;

                        self.another_cycle = true;
                    } else {
                        unreachable!("Sinkable set must be a LocalSet");
                    }
                }
            }
            let effects = EffectAnalyzer::analyze(*expr);
            ctx.check_invalidations(effects);
            return;
        }

        // Handle LocalSet
        let set_index = if let ExpressionKind::LocalSet { index, .. } = &expr.kind {
            Some(*index)
        } else {
            None
        };
        if let Some(index) = set_index {
            if let ExpressionKind::LocalSet { value, .. } = &mut expr.kind {
                self.optimize_function(value, ctx);
            }

            if ctx.sinkables.contains_key(&index) {
                ctx.sinkables.remove(&index);
            }

            let effects = EffectAnalyzer::analyze(*expr);
            ctx.check_invalidations(effects);

            if ctx.can_sink(&*expr) {
                if let ExpressionKind::LocalSet { value, .. } = &expr.kind {
                    let value_effects = EffectAnalyzer::analyze(*value);
                    ctx.sinkables.insert(
                        index,
                        SinkableInfo {
                            effects: value_effects,
                            set: *expr,
                        },
                    );
                }
            } else if ctx.first_cycle && ctx.allow_tee {
                // If we couldn't sink only because it was the first cycle (tee needed),
                // trigger another cycle.
                let use_count = ctx.get_counts.get(&index).copied().unwrap_or(0);
                if use_count > 1 {
                    self.another_cycle = true;
                }
            }
            return;
        }

        // Handle LocalTee
        let tee_vals = if let ExpressionKind::LocalTee { index, .. } = &expr.kind {
            Some(*index)
        } else {
            None
        };
        if tee_vals.is_some() {
            if let ExpressionKind::LocalTee { value, .. } = &mut expr.kind {
                self.optimize_function(value, ctx);
            }
            let effects = EffectAnalyzer::analyze(*expr);
            ctx.check_invalidations(effects);
            return;
        }

        // Handle Drop
        let is_drop = matches!(expr.kind, ExpressionKind::Drop { .. });
        if is_drop {
            if let ExpressionKind::Drop { value } = &mut expr.kind {
                self.optimize_function(value, ctx);
            }
            self.visit_drop_post(expr, ctx);
            let effects = EffectAnalyzer::analyze(*expr);
            ctx.check_invalidations(effects);
            return;
        }

        // Handle Control Flow and Others
        match &mut expr.kind {
            ExpressionKind::Block { list, .. } => {
                for child in list.iter_mut() {
                    self.optimize_function(child, ctx);
                }
                self.visit_block_post(expr, ctx);
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                self.optimize_function(condition, ctx);

                let snapshot = ctx.sinkables.clone();
                self.optimize_function(if_true, ctx);
                ctx.sinkables.retain(|k, v| snapshot.get(k) == Some(v));

                if let Some(false_branch) = if_false {
                    let snapshot = ctx.sinkables.clone();
                    self.optimize_function(false_branch, ctx);
                    ctx.sinkables.retain(|k, v| snapshot.get(k) == Some(v));
                }

                self.visit_if_post(expr, ctx);
            }
            ExpressionKind::Loop { body, .. } => {
                let snapshot = ctx.sinkables.clone();
                self.optimize_function(body, ctx);
                ctx.sinkables.retain(|k, v| snapshot.get(k) == Some(v));

                self.visit_loop_post(expr, ctx);
            }
            ExpressionKind::LocalGet { .. }
            | ExpressionKind::LocalSet { .. }
            | ExpressionKind::LocalTee { .. }
            | ExpressionKind::Drop { .. } => {
                unreachable!("Handled above")
            }
            _ => {
                visit_children(expr, |child| self.optimize_function(child, ctx));
                let effects = EffectAnalyzer::analyze(*expr);
                ctx.check_invalidations(effects);
            }
        }
    }

    fn visit_block_post<'a>(&mut self, expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        if !self.allow_structure {
            return;
        }

        // Check if Block ends with LocalSet
        // Only handle unnamed blocks for now to avoid break target analysis complexity
        match &mut expr.kind {
            ExpressionKind::Block { name, list } if name.is_none() => {
                if let Some(last) = list.last_mut() {
                    if let ExpressionKind::LocalSet { index, value: _ } = &last.kind {
                        let index = *index;

                        // We found a sinkable set at the end of the block.
                        // Transform: (block ... (local.set x val)) -> (local.set x (block ... val))

                        // 1. Get reference to the last element (the Set)
                        let last_idx = list.len() - 1;
                        let set_expr = list[last_idx]; // Copy ExprRef (pointer)

                        // 2. Extract Value from Set
                        // We need access to the set's value to put it in the list
                        let val_expr =
                            if let ExpressionKind::LocalSet { value, .. } = &set_expr.kind {
                                *value
                            } else {
                                unreachable!()
                            };

                        // 3. Update Block list to end with Value instead of Set
                        list[last_idx] = val_expr;

                        // 4. Update Block type to Value type (or Set's type, usually void, but here we propagate value)
                        let new_block_type = val_expr.type_;

                        // 5. Structure Swap: Recycle the Set node to become the Block
                        // `expr` is the Block. `set_expr` is the Set.
                        // We want `expr` to become Set, pointing to `set_expr` which becomes Block.
                        // But wait, `set_expr` is allocated memory. `expr` is allocated memory.
                        // We can swap their contents.

                        let old_block_kind = std::mem::replace(&mut expr.kind, ExpressionKind::Nop);
                        // `expr` is now Nop. We hold Block data in old_block_kind.

                        // 6. Update `set_expr` memory to hold the Block data
                        // We use unsafe to write to the pointer location of `set_expr`
                        unsafe {
                            let ptr = set_expr.as_ptr();
                            // Overwrite Set with Block
                            (*ptr).kind = old_block_kind;
                            (*ptr).type_ = new_block_type;
                        }

                        // 7. Update `expr` to be the LocalSet pointing to `set_expr` (now block)
                        expr.kind = ExpressionKind::LocalSet {
                            index,
                            value: set_expr,
                        };
                        // expr.type_ should remain void (Set type)

                        self.another_cycle = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_if_post<'a>(&mut self, expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        if !self.allow_structure {
            return;
        }

        let match_result = if let ExpressionKind::If {
            if_true,
            if_false: Some(false_branch),
            ..
        } = &mut expr.kind
        {
            if let (
                ExpressionKind::LocalSet { index: idx1, .. },
                ExpressionKind::LocalSet { index: idx2, .. },
            ) = (&if_true.kind, &false_branch.kind)
            {
                if idx1 == idx2 {
                    Some((*idx1, *if_true, *false_branch))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some((target_index, true_set, false_set)) = match_result {
            // Optimization possible:
            // (if (cond) (set x v1) (set x v2)) -> (set x (if (cond) v1 v2))

            // 1. Extract values
            let val1 = if let ExpressionKind::LocalSet { value, .. } = &true_set.kind {
                *value
            } else {
                unreachable!()
            };
            let val2 = if let ExpressionKind::LocalSet { value, .. } = &false_set.kind {
                *value
            } else {
                unreachable!()
            };
            let new_if_type = val1.type_; // Assuming v1 and v2 types match if generic valid wasm

            // 2. Extract Block (If) Kind
            let old_if_kind = std::mem::replace(&mut expr.kind, ExpressionKind::Nop);

            // 3. Reuse `true_set` memory for the If node
            unsafe {
                let ptr = true_set.as_ptr();
                // Reconstruct If with new children
                if let ExpressionKind::If { condition, .. } = old_if_kind {
                    (*ptr).kind = ExpressionKind::If {
                        condition,
                        if_true: val1,
                        if_false: Some(val2),
                    };
                    (*ptr).type_ = new_if_type;
                }
            }

            // 4. Update parent `expr` to be LocalSet
            expr.kind = ExpressionKind::LocalSet {
                index: target_index,
                value: true_set,
            };
            // expr.type_ remains void

            self.another_cycle = true;
        }
    }

    fn visit_loop_post<'a>(&mut self, _expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        // TODO: Implement loop return value optimization (Phase 5b)
    }

    fn visit_drop_post<'a>(&mut self, expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        // Collapse drop-tee into set (drop (local.tee) -> local.set)
        if let ExpressionKind::Drop { value } = &mut expr.kind {
            if let ExpressionKind::LocalTee {
                index,
                value: tee_val,
            } = &value.kind
            {
                let index = *index;
                let tee_val = *tee_val;
                expr.kind = ExpressionKind::LocalSet {
                    index,
                    value: tee_val,
                };
                self.another_cycle = true;
            }
        }
    }
}

/// Count local.get operations in an expression tree
fn count_gets(expr: &Expression, counts: &mut HashMap<u32, usize>) {
    match &expr.kind {
        ExpressionKind::LocalGet { index } => {
            *counts.entry(*index).or_insert(0) += 1;
        }
        _ => {
            visit_children_ref(expr, |child| count_gets(child, counts));
        }
    }
}

/// Visit children of an expression (mutable)
fn visit_children<'a, F>(expr: &mut ExprRef<'a>, mut f: F)
where
    F: FnMut(&mut ExprRef<'a>),
{
    match &mut expr.kind {
        ExpressionKind::Block { list, .. } => {
            for child in list.iter_mut() {
                f(child);
            }
        }
        ExpressionKind::If {
            condition,
            if_true,
            if_false,
        } => {
            f(condition);
            f(if_true);
            if let Some(else_expr) = if_false {
                f(else_expr);
            }
        }
        ExpressionKind::Loop { body, .. } => {
            f(body);
        }
        ExpressionKind::Binary { left, right, .. } => {
            f(left);
            f(right);
        }
        ExpressionKind::Unary { value, .. } => {
            f(value);
        }
        ExpressionKind::LocalSet { value, .. } | ExpressionKind::LocalTee { value, .. } => {
            f(value);
        }
        ExpressionKind::GlobalSet { value, .. } => {
            f(value);
        }
        ExpressionKind::Load { ptr, .. } => {
            f(ptr);
        }
        ExpressionKind::Store { ptr, value, .. } => {
            f(ptr);
            f(value);
        }
        ExpressionKind::Return { value: Some(v) } => {
            f(v);
        }
        ExpressionKind::Drop { value } => {
            f(value);
        }
        ExpressionKind::Select {
            condition,
            if_true,
            if_false,
        } => {
            f(condition);
            f(if_true);
            f(if_false);
        }
        ExpressionKind::Call { operands, .. } | ExpressionKind::CallIndirect { operands, .. } => {
            for operand in operands.iter_mut() {
                f(operand);
            }
        }
        // Leaf nodes or already handled
        ExpressionKind::Nop
        | ExpressionKind::Unreachable
        | ExpressionKind::Const(_)
        | ExpressionKind::LocalGet { .. }
        | ExpressionKind::GlobalGet { .. } => {}
        // TODO: Add more expression types as needed
        _ => {}
    }
}

/// Visit children of an expression (immutable)
fn visit_children_ref<F>(expr: &Expression, mut f: F)
where
    F: FnMut(&Expression),
{
    match &expr.kind {
        ExpressionKind::Block { list, .. } => {
            for child in list.iter() {
                f(child);
            }
        }
        ExpressionKind::If {
            condition,
            if_true,
            if_false,
        } => {
            f(condition);
            f(if_true);
            if let Some(else_expr) = if_false {
                f(else_expr);
            }
        }
        ExpressionKind::Loop { body, .. } => {
            f(body);
        }
        ExpressionKind::Binary { left, right, .. } => {
            f(left);
            f(right);
        }
        ExpressionKind::Unary { value, .. } => {
            f(value);
        }
        ExpressionKind::LocalSet { value, .. } | ExpressionKind::LocalTee { value, .. } => {
            f(value);
        }
        ExpressionKind::GlobalSet { value, .. } => {
            f(value);
        }
        ExpressionKind::Load { ptr, .. } => {
            f(ptr);
        }
        ExpressionKind::Store { ptr, value, .. } => {
            f(ptr);
            f(value);
        }
        ExpressionKind::Return { value: Some(v) } => {
            f(v);
        }
        ExpressionKind::Drop { value } => {
            f(value);
        }
        ExpressionKind::Select {
            condition,
            if_true,
            if_false,
        } => {
            f(condition);
            f(if_true);
            f(if_false);
        }
        ExpressionKind::Call { operands, .. } | ExpressionKind::CallIndirect { operands, .. } => {
            for operand in operands.iter() {
                f(operand);
            }
        }
        // Leaf nodes
        ExpressionKind::Nop
        | ExpressionKind::Unreachable
        | ExpressionKind::Const(_)
        | ExpressionKind::LocalGet { .. }
        | ExpressionKind::GlobalGet { .. } => {}
        // TODO: Add more expression types
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::IrBuilder;
    use binaryen_core::{Literal, Type};
    use bumpalo::collections::Vec as BumpVec;
    use bumpalo::Bump;

    #[test]
    fn test_simplify_locals_basic() {
        // Test that SimplifyLocals can be created and run
        let mut pass = SimplifyLocals::new();
        assert_eq!(pass.name(), "SimplifyLocals");

        // Create empty module
        let bump = Bump::new();
        let mut module = Module::new(&bump);
        pass.run(&mut module);
    }

    #[test]
    fn test_simplify_locals_with_options() {
        // Test different option combinations
        let pass1 = SimplifyLocals::with_options(true, true, true);
        assert!(pass1.allow_tee && pass1.allow_structure && pass1.allow_nesting);

        let pass2 = SimplifyLocals::flat();
        assert!(!pass2.allow_tee && !pass2.allow_structure && !pass2.allow_nesting);
    }

    #[test]
    fn test_function_context() {
        // Test FunctionContext creation
        let ctx = FunctionContext::new(true, true, true);
        assert!(ctx.allow_tee);
        assert!(ctx.allow_structure);
        assert!(ctx.allow_nesting);
        assert!(ctx.first_cycle);
    }

    #[test]
    fn test_sink_to_tee() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new(&bump);

        // (block
        //   (local.set 0 (const 42))
        //   (drop (local.get 0))
        //   (drop (local.get 0))
        // )
        let mut list = BumpVec::new_in(&bump);
        list.push(builder.local_set(0, builder.const_(Literal::I32(42))));
        list.push(builder.drop(builder.local_get(0, Type::I32)));
        list.push(builder.drop(builder.local_get(0, Type::I32)));

        let block = builder.block(None, list, Type::NONE);

        module.functions.push(crate::module::Function {
            name: "test".to_string(),
            type_idx: None,
            params: Type::NONE,
            results: Type::NONE,
            vars: vec![Type::I32],
            body: Some(block),
        });

        let mut pass = SimplifyLocals::new();
        pass.run(&mut module);

        let body = module.functions[0]
            .body
            .expect("Function should have a body");
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert_eq!(list.len(), 3);
            // After all cycles:
            // 1. (nop) - the original set was sunk
            assert!(matches!(list[0].kind, ExpressionKind::Nop));
            // 2. (nop) - the sunk set (from tee) was sunk again
            assert!(matches!(list[1].kind, ExpressionKind::Nop));
            // 3. (drop (const 42)) - final destination
            if let ExpressionKind::Drop { value } = &list[2].kind {
                assert!(matches!(
                    value.kind,
                    ExpressionKind::Const(Literal::I32(42))
                ));
            } else {
                panic!("Expected Drop(Const), got {:?}", list[2].kind);
            }
        } else {
            panic!("Expected Block, got {:?}", body.kind);
        }
    }

    #[test]
    fn test_structure_opt() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new(&bump);

        // (if (local.get 1) (local.set 0 (const 1)) (local.set 0 (const 2)))
        // -> (local.set 0 (if (local.get 1) (const 1) (const 2)))

        let condition = builder.local_get(1, Type::I32);
        let if_true = builder.local_set(0, builder.const_(Literal::I32(1)));
        let if_false = Some(builder.local_set(0, builder.const_(Literal::I32(2))));

        let if_expr = builder.if_(condition, if_true, if_false, Type::NONE);

        module.functions.push(crate::module::Function {
            name: "test".to_string(),
            type_idx: None,
            params: Type::NONE,
            results: Type::NONE,
            vars: vec![Type::I32, Type::I32],
            body: Some(if_expr),
        });

        let mut pass = SimplifyLocals::new();
        pass.run(&mut module);

        let body = module.functions[0]
            .body
            .expect("Function should have a body");
        if let ExpressionKind::LocalSet { index, value } = &body.kind {
            assert_eq!(*index, 0);
            assert!(matches!(value.kind, ExpressionKind::If { .. }));
        } else {
            panic!("Expected LocalSet, got {:?}", body.kind);
        }
    }
}
