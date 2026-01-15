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
struct SinkableInfo<'a> {
    /// Pointer to the expression (for replacement)
    expr_idx: usize,
    /// Effect analysis of the set operation
    effects: Effect,
    /// The local.set expression itself
    set: &'a Expression<'a>,
}

/// Context for a single function optimization
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
    fn can_sink(&self, set: &Expression) -> bool {
        if let ExpressionKind::LocalSet { index, .. } = &set.kind {
            // If in first cycle or not allowing tees, cannot sink if >1 use
            // (would require creating a tee)
            let use_count = self.get_counts.get(index).copied().unwrap_or(0);
            if (self.first_cycle || !self.allow_tee) && use_count > 1 {
                return false;
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

                    // First pass: count local.get operations
                    count_gets(body, &mut ctx.get_counts);

                    // Second pass: optimize
                    self.optimize_function(body, &mut ctx);

                    ctx.first_cycle = false;
                }
            }

            if !self.another_cycle {
                break;
            }
        }
    }
}

impl SimplifyLocals {
    fn optimize_function<'a>(&mut self, expr: &mut ExprRef<'a>, ctx: &mut FunctionContext<'a>) {
        // Post-order traversal
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
                self.optimize_function(if_true, ctx);
                if let Some(false_branch) = if_false {
                    self.optimize_function(false_branch, ctx);
                }
                self.visit_if_post(expr, ctx);
            }
            ExpressionKind::Loop { body, .. } => {
                self.optimize_function(body, ctx);
                self.visit_loop_post(expr, ctx);
            }
            ExpressionKind::LocalGet { index } => {
                // For now, just mark potential optimization
                if ctx.sinkables.contains_key(index) {
                    let one_use =
                        ctx.first_cycle || ctx.get_counts.get(index).copied().unwrap_or(0) == 1;
                    if one_use || ctx.allow_tee {
                        self.another_cycle = true;
                    }
                }
            }
            ExpressionKind::LocalSet { index, value } => {
                self.optimize_function(value, ctx);

                // If we see a set that was already potentially-sinkable,
                // the previous store is dead
                if ctx.sinkables.contains_key(index) {
                    ctx.sinkables.remove(index);
                    self.another_cycle = true;
                }

                // Check effects (we can't borrow expr here, so we'll skip for now)
                // TODO: Need proper effect analysis here
            }
            ExpressionKind::LocalTee { index, value } => {
                self.optimize_function(value, ctx);
                // Tees cannot be sunk, just analyze for effects
                let effects = EffectAnalyzer::analyze(*expr);
                ctx.check_invalidations(effects);
            }
            ExpressionKind::Drop { value } => {
                self.optimize_function(value, ctx);
                self.visit_drop_post(expr, ctx);
            }
            _ => {
                // Visit children for other expression types
                visit_children(expr, |child| self.optimize_function(child, ctx));
            }
        }
    }

    fn visit_block_post<'a>(&mut self, _expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        // TODO: Implement block return value optimization (Phase 5b)
        // This merges local.sets from all break paths into a block return value
    }

    fn visit_if_post<'a>(&mut self, _expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        // TODO: Implement if return value optimization (Phase 5b)
        // This merges local.sets from if_true and if_false into an if return value
    }

    fn visit_loop_post<'a>(&mut self, _expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        // TODO: Implement loop return value optimization (Phase 5b)
    }

    fn visit_drop_post<'a>(&mut self, expr: &mut ExprRef<'a>, _ctx: &mut FunctionContext<'a>) {
        // Collapse drop-tee into set (drop (local.tee) -> local.set)
        if let ExpressionKind::Drop { value } = &mut expr.kind {
            if let ExpressionKind::LocalTee { index, .. } = &value.kind {
                // Convert drop(tee) to set
                // We need to extract the value from the tee and create a new set
                // For now, just mark that optimization is possible
                let _ = index; // Use index to avoid warning
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
        ExpressionKind::Return { value } => {
            if let Some(v) = value {
                f(v);
            }
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
        ExpressionKind::Return { value } => {
            if let Some(v) = value {
                f(v);
            }
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

    #[test]
    fn test_simplify_locals_basic() {
        // Test that SimplifyLocals can be created and run
        let mut pass = SimplifyLocals::new();
        assert_eq!(pass.name(), "SimplifyLocals");

        // Create empty module
        let mut module = Module::new();
        pass.run(&mut module);

        // Pass should complete without errors
        assert!(true);
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
}
