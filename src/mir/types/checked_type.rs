use crate::{
    ast::Span,
    compile::interner::StringId,
    globals::COMMON_IDENTIFIERS,
    mir::types::{
        checked_declaration::{CheckedParam, FnType},
        ordered_float::{OrderedF32, OrderedF64},
    },
    tokenize::NumberKind,
};
use std::{cmp::Ordering, collections::BTreeSet, hash::Hash};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StructKind {
    // packed
    UserDefined(Vec<CheckedParam>),

    /// { id: u16, value: TaglessUnion }
    TaggedUnion(BTreeSet<Type>),

    /// { len: usize, cap: usize, ptr: ptr<T> }
    ListHeader(Box<Type>),

    /// { len: usize, cap: usize, ptr: ptr<u8> }
    StringHeader(Option<StringId>),
}

impl StructKind {
    pub fn fields(&self) -> Vec<(StringId, Type)> {
        match self {
            StructKind::UserDefined(params) => params
                .iter()
                .map(|p| (p.identifier.name, p.ty.kind.clone()))
                .collect(),

            StructKind::ListHeader(elem_ty) => vec![
                (COMMON_IDENTIFIERS.len, Type::USize(None)),
                (COMMON_IDENTIFIERS.cap, Type::USize(None)),
                (COMMON_IDENTIFIERS.ptr, Type::Pointer(elem_ty.clone())),
            ],

            StructKind::StringHeader(_) => vec![
                (COMMON_IDENTIFIERS.len, Type::USize(None)),
                (COMMON_IDENTIFIERS.cap, Type::USize(None)),
                (
                    COMMON_IDENTIFIERS.ptr,
                    Type::Pointer(Box::new(Type::U8(None))),
                ),
            ],
            StructKind::TaggedUnion(variants) => {
                if variants.len() < 2 {
                    panic!(
                        "INTERNAL COMPILER ERROR: Unflattened or empty Union detected. \
                         Always use Type::make_union()"
                    );
                }

                vec![
                    (COMMON_IDENTIFIERS.id, Type::U16(None)),
                    (COMMON_IDENTIFIERS.val, Type::TaglessUnion(variants.clone())),
                ]
            }
        }
    }

    /// Maps a field name -> (Index, Type).
    pub fn get_field(&self, name: &StringId) -> Option<(usize, Type)> {
        self.fields()
            .into_iter()
            .enumerate()
            .find(|(_, (field_name, _))| field_name == name)
            .map(|(index, (_, ty))| (index, ty))
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
    Pointer(Box<Type>),
    Struct(StructKind),
    TaglessUnion(BTreeSet<Type>),
    Fn(FnType),
}

impl Type {
    pub fn unwrap_ptr(&self) -> &Type {
        if let Type::Pointer(to) = self {
            return to;
        }

        panic!("INTERNAL COMPILER ERROR: Called unwrap_ptr on non-pointer type")
    }

    pub fn from_number_kind(val: &NumberKind) -> Type {
        match *val {
            NumberKind::I64(v) => Type::I64(Some(v)),
            NumberKind::I32(v) => Type::I32(Some(v)),
            NumberKind::I16(v) => Type::I16(Some(v)),
            NumberKind::I8(v) => Type::I8(Some(v)),
            NumberKind::F32(v) => Type::F32(Some(OrderedF32(v))),
            NumberKind::F64(v) => Type::F64(Some(OrderedF64(v))),
            NumberKind::U64(v) => Type::U64(Some(v)),
            NumberKind::U32(v) => Type::U32(Some(v)),
            NumberKind::U16(v) => Type::U16(Some(v)),
            NumberKind::U8(v) => Type::U8(Some(v)),
            NumberKind::ISize(v) => Type::ISize(Some(v)),
            NumberKind::USize(v) => Type::USize(Some(v)),
        }
    }

    pub fn make_union(types: impl IntoIterator<Item = Type>) -> Type {
        let mut flat = BTreeSet::new();

        for ty in types {
            if matches!(ty, Type::Never) {
                continue;
            }
            if let Type::Struct(StructKind::TaggedUnion(variants)) = ty {
                flat.extend(variants);
            } else {
                flat.insert(ty);
            }
        }

        if flat.is_empty() {
            return Type::Never;
        }

        if flat.len() == 1 {
            return flat.into_iter().next().unwrap();
        }

        Type::Struct(StructKind::TaggedUnion(flat))
    }

    pub fn union(self, other: Type) -> Type {
        Type::make_union(vec![self, other])
    }

    pub fn intersect(self, other: Type) -> Type {
        let s1 = self.into_set();
        let s2 = other.into_set();
        let result = s1.intersection(&s2).cloned();

        Type::make_union(result)
    }

    pub fn subtract(self, other: Type) -> Type {
        let mut s1 = self.into_set();
        let s2 = other.into_set();

        for t in s2 {
            s1.remove(&t);
        }

        Type::make_union(s1)
    }

    fn into_set(self) -> BTreeSet<Type> {
        if matches!(self, Type::Never) {
            return BTreeSet::new();
        }
        if let Type::Struct(StructKind::TaggedUnion(variants)) = self {
            return variants;
        }

        BTreeSet::from([self])
    }

    pub fn get_union_variants(&self) -> Option<&BTreeSet<Type>> {
        if let Type::Struct(StructKind::TaggedUnion(variants)) = self {
            Some(variants)
        } else {
            None
        }
    }

    /// Maps a struct field name -> (Index, Type)
    pub fn get_field(&self, name: &StringId) -> Option<(usize, Type)> {
        match self {
            Type::Struct(kind) => kind.get_field(name),
            _ => None,
        }
    }

    /// Helper to strip the literal value, returning the generic type.
    /// e.g., I32(Some(5)) -> I32(None)
    pub fn widen(&self) -> Self {
        match self {
            Type::Bool(_) => Type::Bool(None),
            Type::U8(_) => Type::U8(None),
            Type::U16(_) => Type::U16(None),
            Type::U32(_) => Type::U32(None),
            Type::U64(_) => Type::U64(None),
            Type::USize(_) => Type::USize(None),
            Type::ISize(_) => Type::ISize(None),
            Type::I8(_) => Type::I8(None),
            Type::I16(_) => Type::I16(None),
            Type::I32(_) => Type::I32(None),
            Type::I64(_) => Type::I64(None),
            Type::F32(_) => Type::F32(None),
            Type::F64(_) => Type::F64(None),

            Type::Struct(StructKind::StringHeader(_)) => {
                Type::Struct(StructKind::StringHeader(None))
            }

            _ => self.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpannedType {
    pub kind: Type,
    pub span: Span,
}

impl Hash for SpannedType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
    }
}

impl Eq for SpannedType {}
impl PartialEq for SpannedType {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Ord for SpannedType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind.cmp(&other.kind)
    }
}

impl PartialOrd for SpannedType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
