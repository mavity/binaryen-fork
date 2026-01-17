use crate::expression::ExprRef;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Annotation<'a> {
    Loop(LoopType),
    Type(HighLevelType),
    Variable(VariableRole),
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

pub type AnnotationStore<'a> = HashMap<ExprRef<'a>, Annotation<'a>>;
