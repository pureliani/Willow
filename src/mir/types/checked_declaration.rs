use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use crate::{
    ast::{
        decl::{FnDecl, TypeAliasDecl},
        DeclarationId, IdentifierNode, Span,
    },
    compile::interner::TypeId,
    mir::{
        builders::{CheckedFunctionDecl, ValueId},
        types::checked_type::SpannedType,
        utils::scope::Scope,
    },
    parse::DocAnnotation,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CheckedParam {
    pub identifier: IdentifierNode,
    pub ty: SpannedType,
}

impl Ord for CheckedParam {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ty.cmp(&other.ty) {
            Ordering::Equal => self.identifier.cmp(&other.identifier),
            other_order => other_order,
        }
    }
}

impl PartialOrd for CheckedParam {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FnType {
    pub params: Vec<CheckedParam>,
    pub return_type: SpannedType,
}

#[derive(Clone, Debug)]
pub struct CheckedTypeAliasDecl {
    pub id: DeclarationId,
    pub identifier: IdentifierNode,
    pub documentation: Option<DocAnnotation>,
    pub value: Box<SpannedType>,
    pub is_exported: bool,
    pub span: Span,
}

impl Eq for CheckedTypeAliasDecl {}
impl PartialEq for CheckedTypeAliasDecl {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Hash for CheckedTypeAliasDecl {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.identifier.hash(state);
        self.value.hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct CheckedVarDecl {
    pub id: DeclarationId,
    pub identifier: IdentifierNode,
    pub documentation: Option<DocAnnotation>,
    pub constraint_span: Span,
    pub stack_ptr: ValueId,
}

#[derive(Clone, Debug)]
pub enum GenericDeclaration {
    TypeAlias {
        decl: TypeAliasDecl,
        decl_scope: Scope,
    },
    Function {
        decl: FnDecl,
        decl_scope: Scope,
    },
}

#[derive(Clone, Debug)]
pub enum CheckedDeclaration {
    TypeAlias(CheckedTypeAliasDecl),
    Function(CheckedFunctionDecl),
    Var(CheckedVarDecl),
}

#[derive(Debug, Clone)]
pub struct ParamMutation {
    pub param_index: usize,
    pub exit_type: TypeId,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionEffects {
    pub mutations: Vec<ParamMutation>,
}
