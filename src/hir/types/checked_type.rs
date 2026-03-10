use crate::{
    ast::Span,
    compile::interner::StringId,
    hir::types::{
        checked_declaration::{CheckedParam, FnType},
        ordered_number_kind::OrderedNumberKind,
    },
    tokenize::NumberKind,
};
use std::{cmp::Ordering, collections::BTreeSet, hash::Hash};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiteralType {
    Number(OrderedNumberKind),
    Bool(bool),
    String(StringId),
}

impl LiteralType {
    pub fn widen(&self) -> &Type {
        match self {
            LiteralType::Bool(_) => &Type::Bool,
            LiteralType::String(_) => &Type::String,
            LiteralType::Number(n) => match n.0 {
                NumberKind::I64(_) => &Type::I64,
                NumberKind::I32(_) => &Type::I32,
                NumberKind::I16(_) => &Type::I16,
                NumberKind::I8(_) => &Type::I8,
                NumberKind::U64(_) => &Type::U64,
                NumberKind::U32(_) => &Type::U32,
                NumberKind::U16(_) => &Type::U16,
                NumberKind::U8(_) => &Type::U8,
                NumberKind::F64(_) => &Type::F64,
                NumberKind::F32(_) => &Type::F32,
                NumberKind::ISize(_) => &Type::ISize,
                NumberKind::USize(_) => &Type::USize,
            },
        }
    }
}

// TODO: make cheaper to clone
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Type {
    Void,
    Bool,
    U8,
    U16,
    U32,
    U64,
    USize,
    ISize,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Null,
    Literal(LiteralType),
    Struct(Vec<CheckedParam>),
    Union {
        base: BTreeSet<Type>,
        narrowed: BTreeSet<Type>,
    },
    List(Box<SpannedType>),
    String,
    Fn(FnType),
    Unknown,
    Never,
}

impl Type {
    pub fn from_number_kind(val: &NumberKind) -> Type {
        match val {
            NumberKind::I64(_) => Type::I64,
            NumberKind::I32(_) => Type::I32,
            NumberKind::I16(_) => Type::I16,
            NumberKind::I8(_) => Type::I8,
            NumberKind::F32(_) => Type::F32,
            NumberKind::F64(_) => Type::F64,
            NumberKind::U64(_) => Type::U64,
            NumberKind::U32(_) => Type::U32,
            NumberKind::U16(_) => Type::U16,
            NumberKind::U8(_) => Type::U8,
            NumberKind::ISize(_) => Type::ISize,
            NumberKind::USize(_) => Type::USize,
        }
    }

    pub fn make_union(types: impl IntoIterator<Item = Type>) -> Type {
        let mut base = BTreeSet::new();
        let mut narrowed = BTreeSet::new();

        for ty in types {
            if matches!(ty, Type::Never) {
                continue;
            }
            if let Type::Union {
                base: b,
                narrowed: n,
            } = ty
            {
                base.extend(b);
                narrowed.extend(n);
            } else {
                base.insert(ty.clone());
                narrowed.insert(ty);
            }
        }

        if base.is_empty() {
            return Type::Never;
        }

        // CRITICAL: Only unwrap if the PHYSICAL variants collapse to 1.
        if base.len() == 1 {
            return base.into_iter().next().unwrap();
        }

        Type::Union { base, narrowed }
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
        if let Type::Union { narrowed, .. } = self {
            return narrowed;
        }

        let mut set = BTreeSet::new();
        set.insert(self);
        set
    }

    pub fn get_narrowed_variants(&self) -> Option<&BTreeSet<Type>> {
        if let Type::Union { narrowed, .. } = self {
            Some(narrowed)
        } else {
            None
        }
    }

    pub fn get_base_variants(&self) -> Option<&BTreeSet<Type>> {
        if let Type::Union { base, .. } = self {
            Some(base)
        } else {
            None
        }
    }

    /// Maps a struct field name -> (Index, Type)
    pub fn get_field(&self, name: &StringId) -> Option<(usize, &SpannedType)> {
        match self {
            Type::Struct(fields) => fields
                .iter()
                .enumerate()
                .find(|(_, param)| &param.identifier.name == name)
                .map(|(index, param)| (index, &param.ty)),
            _ => None,
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
