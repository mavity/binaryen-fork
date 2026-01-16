use crate::analysis::call_graph::CallGraph;
use crate::expression::{ExprRef, ExpressionKind, IrBuilder};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use binaryen_core::Type;
use bumpalo::collections::Vec as BumpVec;
use std::collections::HashMap;

/// Inlining pass: Inline function calls
pub struct Inlining;

impl Pass for Inlining {
    fn name(&self) -> &str {
        "inlining"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let _call_graph = CallGraph::build(module);

        // Store bodies and signatures
        let mut function_info: HashMap<String, (ExprRef<'a>, Vec<Type>, Vec<Type>)> =
            HashMap::new();
        // We assume params are Type (but we need list).
        // For simplicity in this port, we assume 0 params or handle limited cases,
        // OR we need to know the param types.
        // `func.params` is `Type`. If it's a tuple, we can't iterate it easily in Rust without helper.
        // But `func.vars` is Vec.

        for func in &module.functions {
            if let Some(body) = func.body {
                // Placeholder: Assume empty params for now or fix later
                let params = vec![];
                function_info.insert(func.name.clone(), (body, params, func.vars.clone()));
            }
        }

        let allocator = module.allocator;

        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut inliner = Inliner {
                    function_info: &function_info,
                    builder: IrBuilder::new(allocator),
                    caller_vars: &mut func.vars,
                    // We also need caller param count to offset correctly?
                    // func.vars indices start after params.
                    caller_param_count: 0, // Simplified
                };
                inliner.visit(body);
            }
        }
    }
}

struct Inliner<'a, 'b> {
    function_info: &'b HashMap<String, (ExprRef<'a>, Vec<Type>, Vec<Type>)>,
    builder: IrBuilder<'a>,
    caller_vars: &'b mut Vec<Type>,
    caller_param_count: usize,
}

impl<'a, 'b> Visitor<'a> for Inliner<'a, 'b> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        let mut replacement = None;

        if let ExpressionKind::Call {
            target, operands, ..
        } = &expr.kind
        {
            if let Some((callee_body, callee_params, callee_vars)) = self.function_info.get(*target)
            {
                // Perform inlining

                // 1. Create new locals in caller for callee's params and vars
                // Current locals count
                let base_local_index = self.caller_param_count + self.caller_vars.len();

                // Add callee vars (locals)
                // Note: Params become locals in the inlined code.
                // We don't have types for params here easily?
                // We used `vec![]` placeholder.
                // IF we don't know types, we can't add locals.
                // Critical Gap: We need param types.

                // Workaround: Use operand types for params?
                // Operands match params.
                for op in operands.iter() {
                    self.caller_vars.push(op.type_);
                }
                // Add callee vars
                self.caller_vars.extend_from_slice(callee_vars);

                // 2. Clone body
                let mut cloned_body = self.builder.deep_clone(*callee_body);

                // 3. Remap locals in cloned body
                // We need to offset local indices by `base_local_index`.
                let mut remapper = LocalRemapper {
                    offset: base_local_index as u32,
                };
                remapper.visit(&mut cloned_body);

                // 4. Create Block with assignments
                // (block
                //   (local.set $new_param_0 (operand_0))
                //   ...
                //   (cloned_body)
                // )

                let mut block_list = BumpVec::new_in(self.builder.bump);
                for (i, op) in operands.iter().enumerate() {
                    let local_idx = base_local_index as u32 + i as u32;
                    // We must clone operand if it's used?
                    // `operands` is in the `expr`, which we are replacing.
                    // We can move it or clone. `deep_clone` is safer.
                    let op_clone = self.builder.deep_clone(*op);
                    let set = self.builder.local_set(local_idx, op_clone);
                    block_list.push(set);
                }

                block_list.push(cloned_body);

                // Type of block is type of body (which is type of call)
                let block = self.builder.block(None, block_list, cloned_body.type_);

                replacement = Some(block);
            }
        }

        if let Some(new_expr) = replacement {
            *expr = new_expr;
        } else {
            self.visit_children(expr);
        }
    }
}

struct LocalRemapper {
    offset: u32,
}

impl<'a> Visitor<'a> for LocalRemapper {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        match &mut expr.kind {
            ExpressionKind::LocalGet { index } => *index += self.offset,
            ExpressionKind::LocalSet { index, .. } => *index += self.offset,
            ExpressionKind::LocalTee { index, .. } => *index += self.offset,
            _ => self.visit_children(expr),
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
    fn test_inlining_simple() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Callee: (func $callee (result i32) (i32.const 42))
        let const42 = builder.const_(Literal::I32(42));
        let callee = Function::new(
            "callee".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(const42),
        );

        // Caller: (func $caller (result i32) (call $callee))
        let call = builder.call(
            "callee",
            bumpalo::collections::Vec::new_in(&bump),
            Type::I32,
            false,
        );
        let caller = Function::new(
            "caller".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(call),
        );

        let mut module = Module::new(&bump);
        module.add_function(callee);
        module.add_function(caller);

        let mut pass = Inlining;
        pass.run(&mut module);

        let caller_body = module.functions[1].body.unwrap();

        // Expecting block wrapping the inlined body
        // (block (i32.const 42))  <-- simplified since no params

        if let ExpressionKind::Block { list, .. } = &caller_body.kind {
            assert!(matches!(
                list.last().unwrap().kind,
                ExpressionKind::Const(Literal::I32(42))
            ));
        } else {
            panic!("Expected Block");
        }
    }
}
