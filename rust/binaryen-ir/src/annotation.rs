use crate::expression::ExprRef;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Annotation {
    Loop(LoopType),
    Type(HighLevelType),
    Variable(VariableRole),
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

pub type AnnotationStore<'a> = HashMap<ExprRef<'a>, Annotation>;
