use crate::analysis::cfg::ControlFlowGraph;
use crate::analysis::dominators::DominanceTree;
use crate::analysis::ssa::{DefA, SSABuilder};
use crate::expression::ExpressionKind;
use crate::module::Module;
use crate::pass::Pass;
use binaryen_core::Type;
use std::collections::HashMap;

/// Local Subtyping pass: Refines local types to more specific subtypes using SSA.
pub struct LocalSubtyping;

impl Pass for LocalSubtyping {
    fn name(&self) -> &str {
        "local-subtyping"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        for func in &mut module.functions {
            if let Some(body) = func.body {
                // 1. Build SSA
                let cfg = ControlFlowGraph::build(func, body);
                let dom = DominanceTree::build(&cfg);
                let ssa = SSABuilder::build(func, &cfg, &dom);

                // 2. Map from original local index to the LUB of types assigned to all its SSA versions
                let mut local_lubs: HashMap<u32, Option<Type>> = HashMap::new();

                for (def, _uses) in &ssa.def_uses {
                    match def {
                        DefA::Instruction(instr) => {
                            if let ExpressionKind::LocalSet { index, value }
                            | ExpressionKind::LocalTee { index, value } = &instr.kind
                            {
                                self.note_type(&mut local_lubs, *index, value.type_);
                            }
                        }
                        DefA::Phi(_block, _local_idx) => {
                            // Phis represent a join point.
                            // In a more advanced version, we'd iterate to find a fixpoint for Phis.
                            // For now, we contribute the Phi's current type (calculated during SSA construction).
                            // Wait, Phi doesn't have a type stored in DefA.
                            // But we know which local it belongs to.
                        }
                        DefA::Param(_) => {} // Params are not refinable here
                    }
                }

                // 3. Update local types if LUB is a proper subtype
                let param_count = func.params.tuple_elements().len();
                for (i, var_type) in func.vars.iter_mut().enumerate() {
                    let local_index = (param_count + i) as u32;
                    if let Some(Some(inferred)) = local_lubs.get(&local_index) {
                        if *inferred != *var_type && inferred.is_subtype_of(*var_type) {
                            *var_type = *inferred;
                        }
                    }
                }
            }
        }
    }
}

impl LocalSubtyping {
    fn note_type(&self, lubs: &mut HashMap<u32, Option<Type>>, index: u32, new_type: Type) {
        let entry = lubs.entry(index).or_insert(None);
        if let Some(current) = entry {
            *current = Type::get_lub(*current, new_type);
        } else {
            *entry = Some(new_type);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::IrBuilder;
    use crate::module::{Function, Module};
    use binaryen_core::{Literal, Type};
    use bumpalo::Bump;

    #[test]
    fn test_local_subtyping_noop_mvp() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (local.set 0 (i32.const 42))
        let c42 = builder.const_(Literal::I32(42));
        let set = builder.local_set(0, c42);

        // Function has local 0 as I32
        let func = Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32], // Local 0
            Some(set),
        );

        let bump_mod = Bump::new();
        let mut module = Module::new(&bump_mod);
        module.add_function(func);

        let mut pass = LocalSubtyping;
        pass.run(&mut module);

        // Should remain I32
        assert_eq!(module.functions[0].vars[0], Type::I32);
    }
}
