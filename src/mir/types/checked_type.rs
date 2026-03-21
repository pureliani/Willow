use crate::{
    ast::Span,
    compile::interner::{StringId, TypeId},
    globals::COMMON_IDENTIFIERS,
    mir::types::{
        checked_declaration::{CheckedParam, FnType},
        ordered_float::{OrderedF32, OrderedF64},
    },
};
use std::{cmp::Ordering, collections::BTreeSet, hash::Hash};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StructKind {
    // packed
    UserDefined(Vec<CheckedParam>),

    /// { id: u32, value: TaglessUnion }
    TaggedUnion(BTreeSet<TypeId>),

    /// { len: usize, cap: usize, ptr: ptr<T> }
    ListHeader(TypeId),

    /// { len: usize, cap: usize, ptr: ptr<u8> }
    StringHeader(Option<StringId>),
}

impl StructKind {
    pub fn fields(&self) -> Vec<(StringId, TypeId)> {
        match self {
            StructKind::UserDefined(params) => params
                .iter()
                .map(|p| (p.identifier.name, p.ty.id))
                .collect(),

            StructKind::ListHeader(elem_ty_id) => vec![
                (
                    COMMON_IDENTIFIERS.len,
                    TYPE_INTERNER.intern(&Type::USize(None)),
                ),
                (
                    COMMON_IDENTIFIERS.cap,
                    TYPE_INTERNER.intern(&Type::USize(None)),
                ),
                (
                    COMMON_IDENTIFIERS.ptr,
                    TYPE_INTERNER.intern(&Type::Pointer(*elem_ty_id)),
                ),
            ],

            StructKind::StringHeader(_) => vec![
                (
                    COMMON_IDENTIFIERS.len,
                    TYPE_INTERNER.intern(&Type::USize(None)),
                ),
                (
                    COMMON_IDENTIFIERS.cap,
                    TYPE_INTERNER.intern(&Type::USize(None)),
                ),
                (
                    COMMON_IDENTIFIERS.ptr,
                    TYPE_INTERNER
                        .intern(&Type::Pointer(TYPE_INTERNER.intern(&Type::U8(None)))),
                ),
            ],
            StructKind::TaggedUnion(variants) => {
                vec![
                    (
                        COMMON_IDENTIFIERS.id,
                        TYPE_INTERNER.intern(&Type::U32(None)),
                    ),
                    (
                        COMMON_IDENTIFIERS.val,
                        TYPE_INTERNER.intern(&Type::TaglessUnion(variants.clone())),
                    ),
                ]
            }
        }
    }

    /// Maps a field name -> (Index, TypeId)
    pub fn get_field(&self, name: &StringId) -> Option<(usize, TypeId)> {
        self.fields()
            .into_iter()
            .enumerate()
            .find(|(_, (field_name, _))| field_name == name)
            .map(|(index, (_, ty_id))| (index, ty_id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Type {
    Void,
    Never,
    Unknown,
    Null,
    Bool(Option<bool>),
    U8(Option<u8>),
    U16(Option<u16>),
    U32(Option<u32>),
    U64(Option<u64>),
    USize(Option<usize>),
    ISize(Option<isize>),
    I8(Option<i8>),
    I16(Option<i16>),
    I32(Option<i32>),
    I64(Option<i64>),
    F32(Option<OrderedF32>),
    F64(Option<OrderedF64>),
    Pointer(TypeId),
    Struct(StructKind),
    TaglessUnion(BTreeSet<TypeId>),
    Fn(FnType),
}

#[derive(Clone, Debug)]
pub struct SpannedType {
    pub id: TypeId,
    pub span: Span,
}

impl Hash for SpannedType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for SpannedType {}
impl PartialEq for SpannedType {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ord for SpannedType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for SpannedType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
