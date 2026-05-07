use crate::ast::{expr::Expr, IdentifierNode, Span};

use super::decl::Param;

#[derive(Clone, Debug, PartialEq)]
pub enum TypeAnnotationKind {
    Null,
    Void,
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    ISize,
    USize,
    Identifier(IdentifierNode),
    GenericApply {
        left: Box<TypeAnnotation>,
        args: Vec<TypeAnnotation>,
    },
    Struct(Vec<Param>),
    Pointer(Box<TypeAnnotation>),
    MutPointer(Box<TypeAnnotation>),
    FnType {
        params: Vec<Param>,
        return_type: Box<TypeAnnotation>,
    },
    Refinement {
        base: Box<TypeAnnotation>,
        condition: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeAnnotation {
    pub kind: TypeAnnotationKind,
    pub span: Span,
}
