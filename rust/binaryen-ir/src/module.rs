use crate::expression::ExprRef;
use binaryen_core::Type;

#[derive(Debug)]
pub struct Function<'a> {
    pub name: String,
    pub params: Type,
    pub results: Type,
    pub vars: Vec<Type>,
    pub body: Option<ExprRef<'a>>,
}

impl<'a> Function<'a> {
    pub fn new(
        name: String,
        params: Type,
        results: Type,
        vars: Vec<Type>,
        body: Option<ExprRef<'a>>,
    ) -> Self {
        Self {
            name,
            params,
            results,
            vars,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryLimits {
    pub initial: u32,         // Initial size in 64KB pages
    pub maximum: Option<u32>, // Optional maximum size
}

#[derive(Debug)]
pub struct Global<'a> {
    pub name: String,
    pub type_: Type,
    pub mutable: bool,
    pub init: ExprRef<'a>, // Initialization expression (must be constant)
}

#[derive(Debug, Clone)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Function = 0,
    Table = 1,
    Memory = 2,
    Global = 3,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Function(Type, Type),          // params, results
    Table(Type, u32, Option<u32>), // elem_type, min, max
    Memory(MemoryLimits),
    Global(Type, bool), // type, mutable
}

#[derive(Debug, Default)]
pub struct Module<'a> {
    pub imports: Vec<Import>,
    pub functions: Vec<Function<'a>>,
    pub globals: Vec<Global<'a>>,
    pub memory: Option<MemoryLimits>,
    pub exports: Vec<Export>,
}

impl<'a> Module<'a> {
    pub fn new() -> Self {
        Self {
            imports: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
            memory: None,
            exports: Vec::new(),
        }
    }

    pub fn add_import(&mut self, import: Import) {
        self.imports.push(import);
    }

    pub fn add_function(&mut self, func: Function<'a>) {
        self.functions.push(func);
    }

    pub fn get_function(&self, name: &str) -> Option<&Function<'a>> {
        self.functions.iter().find(|f| f.name == name)
    }

    pub fn get_function_mut(&mut self, name: &str) -> Option<&mut Function<'a>> {
        self.functions.iter_mut().find(|f| f.name == name)
    }

    pub fn set_memory(&mut self, initial: u32, maximum: Option<u32>) {
        self.memory = Some(MemoryLimits { initial, maximum });
    }

    pub fn add_export(&mut self, name: String, kind: ExportKind, index: u32) {
        self.exports.push(Export { name, kind, index });
    }

    pub fn export_function(&mut self, func_index: u32, export_name: String) {
        self.add_export(export_name, ExportKind::Function, func_index);
    }

    pub fn add_global(&mut self, global: Global<'a>) {
        self.globals.push(global);
    }

    pub fn get_global(&self, name: &str) -> Option<&Global<'a>> {
        self.globals.iter().find(|g| g.name == name)
    }

    pub fn export_global(&mut self, global_index: u32, export_name: String) {
        self.add_export(export_name, ExportKind::Global, global_index);
    }
}
