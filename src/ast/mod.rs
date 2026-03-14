use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use crate::compile::interner::StringId;

pub mod decl;
pub mod expr;
pub mod stmt;
pub mod type_annotation;

#[derive(Debug, Clone)]
pub struct IdentifierNode {
    pub name: StringId,
    pub span: Span,
}

impl Eq for IdentifierNode {}
impl PartialEq for IdentifierNode {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Hash for IdentifierNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Ord for IdentifierNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.0.cmp(&other.name.0)
    }
}

impl PartialOrd for IdentifierNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct StringNode {
    pub value: String,
    pub len: usize,
    pub span: Span,
}

impl Eq for StringNode {}
impl PartialEq for StringNode {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl Hash for StringNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModulePath(pub Arc<PathBuf>);

impl From<ModulePath> for PathBuf {
    fn from(value: ModulePath) -> Self {
        value.0.to_path_buf()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy, Default)]
pub struct Position {
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub start: Position,
    pub end: Position,
    pub path: ModulePath,
}

impl Span {
    pub fn contains(&self, byte_offset: usize) -> bool {
        byte_offset >= self.start.byte_offset && byte_offset <= self.end.byte_offset
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeclarationId(pub usize);
