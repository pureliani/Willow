use crate::{
    ast::Span,
    compile::interner::{StringId, TypeId, TypeInterner},
    globals::COMMON_IDENTIFIERS,
    mir::{
        types::{
            checked_declaration::{CheckedParam, FnType},
            ordered_float::{OrderedF32, OrderedF64},
        },
        utils::layout::get_layout_of,
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

#[derive(Clone, Debug)]
pub struct StructField {
    pub name: StringId,
    pub ty: TypeId,
    pub physical_index: Option<usize>,
}

impl StructKind {
    pub fn fields(&self, t: &TypeInterner) -> Vec<StructField> {
        let logical = match self {
            StructKind::UserDefined(params) => params
                .iter()
                .map(|p| (p.identifier.name, p.ty.id))
                .collect(),

            StructKind::ListHeader(elem_ty_id) => vec![
                (COMMON_IDENTIFIERS.len, t.usize(None)),
                (COMMON_IDENTIFIERS.cap, t.usize(None)),
                (COMMON_IDENTIFIERS.ptr, t.ptr(*elem_ty_id)),
            ],

            StructKind::StringHeader(_) => vec![
                (COMMON_IDENTIFIERS.len, t.usize(None)),
                (COMMON_IDENTIFIERS.cap, t.usize(None)),
                (COMMON_IDENTIFIERS.ptr, t.ptr(t.u8(None))),
            ],
            StructKind::TaggedUnion(variants) => vec![
                (COMMON_IDENTIFIERS.id, t.u32(None)),
                (
                    COMMON_IDENTIFIERS.val,
                    t.intern(&Type::TaglessUnion(variants.clone())),
                ),
            ],
        };

        if matches!(self, StructKind::StringHeader(Some(_))) {
            return logical
                .into_iter()
                .map(|(name, ty)| StructField {
                    name,
                    ty,
                    physical_index: None,
                })
                .collect();
        }

        let mut physical_index = 0;
        let mut result = Vec::with_capacity(logical.len());

        for (name, ty_id) in logical {
            let ty = t.resolve(ty_id);

            let is_field_zst = get_layout_of(&ty, t).is_none();

            let current_physical_index = if is_field_zst {
                None
            } else {
                let idx = physical_index;
                physical_index += 1;
                Some(idx)
            };

            result.push(StructField {
                name,
                ty: ty_id,
                physical_index: current_physical_index,
            });
        }

        result
    }

    pub fn get_field(&self, t: &TypeInterner, name: &StringId) -> Option<StructField> {
        self.fields(t).into_iter().find(|f| f.name == *name)
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
