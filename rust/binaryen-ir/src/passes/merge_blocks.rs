use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use bumpalo::collections::Vec as BumpVec;

/// Merge Blocks pass: Combines consecutive blocks that can be merged
///
/// This pass looks for opportunities to flatten nested block structures
/// and merge sequential blocks into single blocks for simpler control flow.
///
/// Examples:
/// - (block (block ...)) => (block ...)
/// - Sequential blocks with no control flow => single block
pub struct MergeBlocks;

impl Pass for MergeBlocks {
    fn name(&self) -> &str {
        "merge-blocks"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let allocator = module.allocator();

        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut merger = BlockMerger { allocator };
                merger.visit(body);
            }
        }
    }
}

struct BlockMerger<'a> {
    allocator: &'a bumpalo::Bump,
}

impl<'a> Visitor<'a> for BlockMerger<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        // Look for nested blocks that can be flattened
        if let ExpressionKind::Block { name, list } = &expr.kind {
            // If this block contains a single block child, flatten it
            if list.len() == 1 {
                if let ExpressionKind::Block {
                    name: inner_name,
                    list: inner_list,
                } = &list[0].kind
                {
                    // Only merge if names are compatible (both None or same name)
                    let can_merge = match (name, inner_name) {
                        (None, _) => true,
                        (Some(n1), Some(n2)) if n1 == n2 => true,
                        _ => false,
                    };

                    if can_merge && !inner_list.is_empty() {
                        // Flatten: replace outer block with inner block's contents
                        let block_type = expr.type_;
                        let merged_name = name.or(*inner_name);

                        let mut new_list = BumpVec::new_in(self.allocator);
                        for item in inner_list.iter() {
                            new_list.push(*item);
                        }

                        *expr = crate::expression::Expression::block(
                            self.allocator,
                            merged_name,
                            new_list,
                            block_type,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind};
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_merge_blocks_single_nested() {
        let bump = Bump::new();

        // Create: (block (block (i32.const 42)))
        let const_val = Expression::const_expr(&bump, Literal::I32(42), Type::I32);

        let mut inner_list = BumpVec::new_in(&bump);
        inner_list.push(const_val);
        let inner_block = Expression::block(&bump, None, inner_list, Type::I32);

        let mut outer_list = BumpVec::new_in(&bump);
        outer_list.push(inner_block);
        let outer_block = Expression::block(&bump, None, outer_list, Type::I32);

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(outer_block),
        );

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = MergeBlocks;
        pass.run(&mut module);

        // Should be flattened to single block
        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert_eq!(list.len(), 1);
            assert!(matches!(list[0].kind, ExpressionKind::Const(_)));
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_merge_blocks_preserves_non_nested() {
        let bump = Bump::new();

        // Create: (block (i32.const 1) (i32.const 2))
        let const1 = Expression::const_expr(&bump, Literal::I32(1), Type::I32);
        let const2 = Expression::const_expr(&bump, Literal::I32(2), Type::I32);

        let mut list = BumpVec::new_in(&bump);
        list.push(const1);
        list.push(const2);
        let block = Expression::block(&bump, None, list, Type::I32);

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(block),
        );

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = MergeBlocks;
        pass.run(&mut module);

        // Should remain unchanged
        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, .. } = &body.kind {
            assert_eq!(list.len(), 2);
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_merge_blocks_preserves_names() {
        let bump = Bump::new();

        // Create: (block $outer (block $inner (i32.const 42)))
        // Should not merge because names differ
        let const_val = Expression::const_expr(&bump, Literal::I32(42), Type::I32);

        let inner_name = bump.alloc_str("inner");
        let mut inner_list = BumpVec::new_in(&bump);
        inner_list.push(const_val);
        let inner_block = Expression::block(&bump, Some(inner_name), inner_list, Type::I32);

        let outer_name = bump.alloc_str("outer");
        let mut outer_list = BumpVec::new_in(&bump);
        outer_list.push(inner_block);
        let outer_block = Expression::block(&bump, Some(outer_name), outer_list, Type::I32);

        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(outer_block),
        );

        let mut module = Module::new(&bump);
        module.add_function(func);

        let mut pass = MergeBlocks;
        pass.run(&mut module);

        // Should NOT merge due to different names
        let body = module.functions[0].body.as_ref().unwrap();
        if let ExpressionKind::Block { list, name } = &body.kind {
            assert_eq!(list.len(), 1);
            assert_eq!(*name, Some("outer"));
            assert!(matches!(list[0].kind, ExpressionKind::Block { .. }));
        }
    }
}
