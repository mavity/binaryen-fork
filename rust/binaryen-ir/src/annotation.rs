use crate::expression::ExprRef;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Annotation<'a> {
    Loop(LoopType),
    Type(HighLevelType),
    Variable(VariableRole),
    If {
        condition: ExprRef<'a>,
        inverted: bool,
    },
    Inlined,                   // Tag for local.set that should be omitted
    InlinedValue(ExprRef<'a>), // Tag for local.get that should be replaced by this expression
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopType {
    For,
    While,
    DoWhile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighLevelType {
    Bool,
    Pointer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRole {
    LoopIndex,
    BasePointer,
}

#[derive(Debug, Default)]
pub struct AnnotationStore<'a> {
    inner: HashMap<ExprRef<'a>, Annotation<'a>>,
}

impl<'a> IntoIterator for AnnotationStore<'a> {
    type Item = (ExprRef<'a>, Annotation<'a>);
    type IntoIter = std::collections::hash_map::IntoIter<ExprRef<'a>, Annotation<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> AnnotationStore<'a> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn insert(&mut self, expr: ExprRef<'a>, annotation: Annotation<'a>) {
        self.inner.insert(expr, annotation);
    }

    pub fn get(&self, expr: ExprRef<'a>) -> Option<&Annotation<'a>> {
        self.inner.get(&expr)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// If this is an 'If' block, returns information about the condition.
    pub fn get_if_info(&self, expr: ExprRef<'a>) -> Option<(ExprRef<'a>, bool)> {
        match self.inner.get(&expr) {
            Some(Annotation::If {
                condition,
                inverted,
            }) => Some((*condition, *inverted)),
            _ => None,
        }
    }

    /// Returns the high-level loop type if this is a loop.
    pub fn get_loop_type(&self, expr: ExprRef<'a>) -> Option<LoopType> {
        match self.inner.get(&expr) {
            Some(Annotation::Loop(lt)) => Some(*lt),
            _ => None,
        }
    }

    /// Returns the high-level type if this expression has one.
    pub fn get_high_level_type(&self, expr: ExprRef<'a>) -> Option<HighLevelType> {
        match self.inner.get(&expr) {
            Some(Annotation::Type(ht)) => Some(*ht),
            _ => None,
        }
    }

    /// Returns true if this expression should be treated as 'inlined' (omitted as a statement).
    pub fn is_inlined(&self, expr: ExprRef<'a>) -> bool {
        matches!(self.inner.get(&expr), Some(Annotation::Inlined))
    }

    /// Returns the source expression if this is an inlined value.
    pub fn get_inlined_value(&self, expr: ExprRef<'a>) -> Option<ExprRef<'a>> {
        match self.inner.get(&expr) {
            Some(Annotation::InlinedValue(val)) => Some(*val),
            _ => None,
        }
    }
}
