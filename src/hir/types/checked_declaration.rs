use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use crate::{
    ast::{
        decl::{FnDecl, TypeAliasDecl},
        type_annotation::TypeAnnotation,
        DeclarationId, IdentifierNode, Span,
    },
    hir::{
        builders::CheckedFunctionDecl, types::checked_type::SpannedType,
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
    pub constraint: Option<TypeAnnotation>,
}

#[derive(Clone, Debug)]
pub enum GenericDeclaration {
    TypeAlias {
        decl: TypeAliasDecl,
        decl_scope: Scope,
        has_errors: bool,
    },
    Function {
        decl: FnDecl,
        decl_scope: Scope,
        has_errors: bool,
    },
}

#[derive(Clone, Debug)]
pub enum CheckedDeclaration {
    TypeAlias(CheckedTypeAliasDecl),
    Function(CheckedFunctionDecl),
    Var(CheckedVarDecl),
}
