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
    pub fn new(name: String, params: Type, results: Type, vars: Vec<Type>, body: Option<ExprRef<'a>>) -> Self {
        Self {
            name,
            params,
            results,
            vars,
            body,
        }
    }
}

#[derive(Debug, Default)]
pub struct Module<'a> {
    pub functions: Vec<Function<'a>>,
}

impl<'a> Module<'a> {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
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
}
