use crate::{
    ast::{expr::BlockContents, DeclarationId, IdentifierNode},
    compile::interner::StringId,
    parse::DocAnnotation,
};

use super::{expr::Expr, type_annotation::TypeAnnotation};

#[derive(Clone, Debug, PartialEq)]
pub struct GenericParam {
    pub identifier: IdentifierNode,
    pub extends: Option<TypeAnnotation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub id: DeclarationId,
    pub identifier: IdentifierNode,
    pub constraint: TypeAnnotation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnDecl {
    pub id: DeclarationId,
    pub documentation: Option<DocAnnotation>,
    pub identifier: IdentifierNode,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: TypeAnnotation,
    pub body: BlockContents,
    pub is_exported: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeAliasDecl {
    pub id: DeclarationId,
    pub documentation: Option<DocAnnotation>,
    pub identifier: IdentifierNode,
    pub generic_params: Vec<GenericParam>,
    pub value: TypeAnnotation,
    pub is_exported: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarDecl {
    pub id: DeclarationId,
    pub documentation: Option<DocAnnotation>,
    pub identifier: IdentifierNode,
    pub constraint: Option<TypeAnnotation>,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Declaration {
    TypeAlias(TypeAliasDecl),
    Fn(FnDecl),
    Var(VarDecl),
    Param(Param),
    GenericParameter(StringId),
}
