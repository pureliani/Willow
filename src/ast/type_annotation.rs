use crate::{
    ast::{IdentifierNode, Span},
    compile::interner::StringId,
};

use super::decl::Param;

#[derive(Clone, Debug, PartialEq)]
pub enum TypeAnnotationKind {
    Null,
    Void,
    Bool(Option<bool>),
    U8(Option<u8>),
    U16(Option<u16>),
    U32(Option<u32>),
    U64(Option<u64>),
    I8(Option<i8>),
    I16(Option<i16>),
    I32(Option<i32>),
    I64(Option<i64>),
    F32(Option<f32>),
    F64(Option<f64>),
    ISize(Option<isize>),
    USize(Option<usize>),
    String(Option<StringId>),
    Identifier(IdentifierNode),
    Struct(Vec<Param>),
    Union(Vec<TypeAnnotation>),
    List(Box<TypeAnnotation>),
    FnType {
        params: Vec<Param>,
        return_type: Box<TypeAnnotation>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeAnnotation {
    pub kind: TypeAnnotationKind,
    pub span: Span,
}
