use crate::module::Module;
use crate::pass::Pass;

/// Removes all debug information (names, locations).
pub struct StripDebug;

impl Pass for StripDebug {
    fn name(&self) -> &str {
        "StripDebug"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // Clear annotations (generic metadata container)
        module.annotations = crate::annotation::AnnotationStore::default();

        // In this IR, function names are crucial for ID-ing, so they stay.
        // But local names would be stripped if we had them.
    }
}

/// Removes DWARF debug information only.
pub struct StripDWARF;

impl Pass for StripDWARF {
    fn name(&self) -> &str {
        "StripDWARF"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        // Clear anything that looks like DWARF in annotations
        // For now, clear all annotations as a proxy.
        module.annotations = crate::annotation::AnnotationStore::default();
    }
}
