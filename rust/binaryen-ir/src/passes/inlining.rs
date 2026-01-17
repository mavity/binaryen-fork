use crate::analysis::call_graph::CallGraph;
use crate::analysis::cost::CostEstimator;
use crate::expression::{ExprRef, ExpressionKind, IrBuilder};
use crate::module::Module;
use crate::pass::{InliningOptions, Pass};
use crate::visitor::Visitor;
use binaryen_core::Type;
use bumpalo::collections::Vec as BumpVec;
use std::collections::HashMap;

/// Inlining pass: Inline function calls
pub struct Inlining {
    pub options: InliningOptions,
}

impl Inlining {
    pub fn new() -> Self {
        Self {
            options: InliningOptions::default(),
        }
    }

    pub fn with_options(options: InliningOptions) -> Self {
        Self { options }
    }

    fn worth_full_inlining(
        &self,
        _callee_name: &str,
        num_callers: usize,
        callee: &crate::module::Function,
    ) -> bool {
        let cost = CostEstimator::inline_cost(callee);

        if cost.has_try_delegate {
            return false;
        }

        if !self.options.allow_functions_with_loops && cost.loop_count > 0 {
            return false;
        }

        // Always inline small functions
        if cost.instruction_count <= self.options.always_inline_max_size {
            return true;
        }

        // Inline if only one caller and not too big
        if num_callers == 1 && cost.instruction_count <= self.options.one_caller_inline_max_size {
            return true;
        }

        // Default heuristic
        if cost.instruction_count <= self.options.default_inline_max_size {
            return true;
        }

        false
    }
}

impl Pass for Inlining {
    fn name(&self) -> &str {
        "inlining"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let call_graph = CallGraph::build(module);

        // Store bodies and signatures
        let mut function_info: HashMap<String, (ExprRef<'a>, Type, Vec<Type>)> = HashMap::new();

        for func in &module.functions {
            if let Some(body) = func.body {
                function_info.insert(func.name.clone(), (body, func.params, func.vars.clone()));
            }
        }

        let allocator = module.allocator;

        // 1. Identification: Which functions are good candidates (Full Inlining)
        let mut candidates = HashMap::new();
        for func in &module.functions {
            let num_callers = call_graph
                .get_callers(&func.name)
                .map(|s| s.len())
                .unwrap_or(0);
            if self.worth_full_inlining(&func.name, num_callers, func) {
                candidates.insert(func.name.clone(), true);
            }
        }

        // 2. Application: Traverse all functions and inline calls to candidates
        for caller_idx in 0..module.functions.len() {
            let (mut body, mut vars, params) = {
                let f = &mut module.functions[caller_idx];
                if f.body.is_none() {
                    continue;
                }
                (f.body.unwrap(), std::mem::take(&mut f.vars), f.params)
            };

            let mut inliner = Inliner {
                function_info: &function_info,
                candidates: &candidates,
                builder: IrBuilder::new(allocator),
                caller_vars: &mut vars,
                caller_param_count: params.tuple_len(),
            };
            inliner.visit(&mut body);

            // Put vars back
            module.functions[caller_idx].vars = vars;
            module.functions[caller_idx].body = Some(body);
        }
    }
}

struct Inliner<'a, 'b> {
    function_info: &'b HashMap<String, (ExprRef<'a>, Type, Vec<Type>)>,
    candidates: &'b HashMap<String, bool>,
    builder: IrBuilder<'a>,
    caller_vars: &'b mut Vec<Type>,
    caller_param_count: usize,
}

impl<'a, 'b> Visitor<'a> for Inliner<'a, 'b> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        // Bottom-up traversal
        self.visit_children(expr);

        if let ExpressionKind::Call {
            target, operands, ..
        } = &expr.kind
        {
            if !self.candidates.contains_key(*target) {
                return;
            }

            if let Some((callee_body, callee_params, callee_vars)) = self.function_info.get(*target)
            {
                // Perform full inlining
                let mut block_list = BumpVec::new_in(self.builder.bump);
                let base_local_index = self.caller_param_count + self.caller_vars.len();

                // Parameters become locals
                let param_types = callee_params.tuple_elements();
                for (i, &param_type) in param_types.iter().enumerate() {
                    self.caller_vars.push(param_type);
                    let operand = operands[i];
                    block_list.push(
                        self.builder
                            .local_set((base_local_index + i) as u32, operand),
                    );
                }

                // Add callee vars (actual locals)
                for &var_type in callee_vars {
                    self.caller_vars.push(var_type);
                }

                // Clone body
                let mut cloned_body = self.builder.deep_clone(*callee_body);

                // Remap locals in cloned body
                let mut remapper = LocalRemapper {
                    offset: base_local_index as u32,
                };
                remapper.visit(&mut cloned_body);

                block_list.push(cloned_body);

                let result_type = cloned_body.type_;
                let block = self.builder.block(None, block_list, result_type);
                *expr = block;
            }
        }
    }
}

struct LocalRemapper {
    offset: u32,
}

impl<'a> Visitor<'a> for LocalRemapper {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        match &mut expr.kind {
            ExpressionKind::LocalGet { index } => {
                *index += self.offset;
            }
            ExpressionKind::LocalSet { index, .. } => {
                *index += self.offset;
            }
            ExpressionKind::LocalTee { index, .. } => {
                *index += self.offset;
            }
            _ => {
                // Do not visit children here if we want to separate visit_expression and visit_children
                // Wait, Visitor trait has visit() which calls visit_expression then visit_children.
                // If we want to change nodes, we should do it in visit_expression.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::ExpressionKind;
    use crate::module::Function;
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_inlining_full_parity() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Callee: (func $callee (param i32) (result i32) (local.get 0))
        let body = builder.local_get(0, Type::I32);
        let callee = Function::new(
            "callee".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(body),
        );

        // Caller: (func $caller (result i32) (call $callee (i32.const 42)))
        let arg = builder.const_(Literal::I32(42));
        let mut args = BumpVec::new_in(&bump);
        args.push(arg);
        let call = builder.call("callee", args, Type::I32, false);
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

        let mut inlining = Inlining::new();
        inlining.run(&mut module);

        let caller_body = module.functions[1].body.unwrap();

        // Expect:
        // (block
        //   (local.set 0 (i32.const 42))
        //   (local.get 0)
        // )
        if let ExpressionKind::Block { list, .. } = &caller_body.kind {
            assert_eq!(list.len(), 2);
            assert!(matches!(
                list[0].kind,
                ExpressionKind::LocalSet { index: 0, .. }
            ));
            assert!(matches!(
                list[1].kind,
                ExpressionKind::LocalGet { index: 0 }
            ));
        } else {
            panic!("Expected Block, got {:?}", caller_body.kind);
        }
    }
}
