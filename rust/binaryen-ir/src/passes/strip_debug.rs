use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;

/// Removes all debug information (names, locations).
pub struct StripDebug;

impl Pass for StripDebug {
    fn name(&self) -> &str {
        "StripDebug"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // Clear all debug names
        for func in &mut module.functions {
            func.local_names.clear();
        }

        // Clear all annotations (includes locations, local names, etc.)
        module.annotations.clear();
    }
}

/// Removes DWARF debug information only.
pub struct StripDWARF;

impl Pass for StripDWARF {
    fn name(&self) -> &str {
        "StripDWARF"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // Clear only location info from annotations
        module.annotations.clear_locations();
    }
}

/// Propagates debug locations from parent to children.
pub struct PropagateDebugLocs;

impl Pass for PropagateDebugLocs {
    fn name(&self) -> &str {
        "PropagateDebugLocs"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let Module {
            functions,
            annotations,
            ..
        } = module;
        let mut propagator = DebugLocPropagator {
            annotations,
            current_loc: None,
        };

        for func in functions {
            if let Some(mut body) = func.body {
                propagator.visit(&mut body);
            }
        }
    }
}

struct DebugLocPropagator<'a, 'm> {
    annotations: &'m mut crate::annotation::AnnotationStore<'a>,
    current_loc: Option<crate::annotation::DebugLocation>,
}

impl<'a, 'm> crate::visitor::Visitor<'a> for DebugLocPropagator<'a, 'm> {
    fn visit_expression(&mut self, expr: &mut crate::expression::ExprRef<'a>) {
        let old_loc = self.current_loc;

        // If this expression has a location, it becomes the new current for children
        if let Some(loc) = self.annotations.get_location(*expr) {
            self.current_loc = Some(loc);
        } else if let Some(loc) = self.current_loc {
            // Otherwise, inherit parent's location
            self.annotations.set_location(*expr, loc);
        }

        self.visit_children(expr);
        self.current_loc = old_loc;
    }
}
