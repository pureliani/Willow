use crate::{
    ast::{DeclarationId, IdentifierNode, Span},
    compile::interner::{StringId, TypeId, TypeInterner},
    globals::COMMON_IDENTIFIERS,
    hir::types::ordered_float::{OrderedF32, OrderedF64},
};
use std::{cmp::Ordering, collections::BTreeSet, hash::Hash};

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

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StructKind {
    // packed
    UserDefined(Vec<CheckedParam>),

    /// { id: u32, value: TaglessUnion }
    TaggedUnion(BTreeSet<TypeId>),

    /// { len: usize, cap: usize, ptr: ptr<T> }
    ListHeader(TypeId),

    String(Option<StringId>),
}

impl StructKind {
    pub fn fields(&self, t: &TypeInterner) -> Vec<(StringId, TypeId)> {
        match self {
            StructKind::UserDefined(params) => params
                .iter()
                .map(|p| (p.identifier.name, p.ty.id))
                .collect(),

            StructKind::ListHeader(elem_ty_id) => vec![
                (COMMON_IDENTIFIERS.len, t.usize(None)),
                (COMMON_IDENTIFIERS.cap, t.usize(None)),
                (COMMON_IDENTIFIERS.ptr, t.ptr(*elem_ty_id)),
            ],

            StructKind::String(literal) => {
                if literal.is_none() {
                    vec![(COMMON_IDENTIFIERS.len, t.usize(None))]
                } else {
                    vec![]
                }
            }
            StructKind::TaggedUnion(variants) => vec![
                (COMMON_IDENTIFIERS.id, t.u32(None)),
                (
                    COMMON_IDENTIFIERS.val,
                    t.intern(&Type::TaglessUnion(variants.clone())),
                ),
            ],
        }
    }

    pub fn get_field(
        &self,
        t: &TypeInterner,
        name: &StringId,
    ) -> Option<(usize, TypeId)> {
        self.fields(t)
            .into_iter()
            .enumerate()
            .find(|(_, (field_name, _))| field_name == name)
            .map(|(index, (_, ty_id))| (index, ty_id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FnTypeKind {
    Indirect(FnType),
    Direct(DeclarationId),
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
    Fn(FnTypeKind),

    Pointer(TypeId),
    Struct(StructKind),
    TaglessUnion(BTreeSet<TypeId>),

    GenericParam {
        identifier: IdentifierNode,
        extends: Option<TypeId>,
    },
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
