use crate::expression::{ExprRef, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use std::collections::HashMap;

/// Minifies names in the module (functions, globals, etc.)
pub struct MinifyNames;

impl Pass for MinifyNames {
    fn name(&self) -> &str {
        "MinifyNames"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let mut name_map = HashMap::new();
        let mut next_id = 0;

        let mut get_next_name = || {
            let mut name = String::new();
            let mut id = next_id;
            loop {
                let char = ((id % 26) as u8 + b'a') as char;
                name.insert(0, char);
                id /= 26;
                if id == 0 {
                    break;
                }
            }
            next_id += 1;
            name
        };

        // 1. Minify function names
        for func in &mut module.functions {
            let old_name = func.name.clone();
            let new_name = get_next_name();
            name_map.insert(old_name, new_name.clone());
            func.name = new_name;
        }

        // 2. Update calls and other name references
        let mut updater = NameUpdater {
            name_map: &name_map,
            allocator: &module.allocator,
        };
        for func in &mut module.functions {
            if let Some(mut body) = func.body {
                updater.visit(&mut body);
            }
        }

        for global in &mut module.globals {
            updater.visit(&mut global.init);
        }
    }
}

/// Removes all names from the module where possible.
pub struct StripNames;

impl Pass for StripNames {
    fn name(&self) -> &str {
        "StripNames"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // In WASM, functions must have names if they are to be called by name in the IR.
        // But we can minify them to nothing or very short.
        // Real StripNames in Binaryen often just clears the Name section in binary.
        // In our IR, names are IDs.

        let mut minifier = MinifyNames;
        minifier.run(module);
    }
}

struct NameUpdater<'map, 'a> {
    name_map: &'map HashMap<String, String>,
    allocator: &'a bumpalo::Bump,
}

impl<'map, 'a> Visitor<'a> for NameUpdater<'map, 'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        match &mut expr.kind {
            ExpressionKind::Call { target, .. } => {
                if let Some(new_name) = self.name_map.get(*target) {
                    *target = self.allocator.alloc_str(new_name);
                }
            }
            _ => {}
        }
        self.visit_children(expr);
    }
}
