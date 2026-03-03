use crate::{
    compile::interner::StringId,
    hir::types::{
        checked_declaration::{CheckedParam, FnType},
        ordered_number_kind::OrderedNumberKind,
    },
    tokenize::NumberKind,
};
use std::{collections::BTreeSet, hash::Hash};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiteralType {
    Number(OrderedNumberKind),
    Bool(bool),
    String(StringId),
}

impl LiteralType {
    pub fn widen<'a>(&'a self) -> &'a Type {
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
    Union(BTreeSet<Type>),
    List(Box<Type>),
    String,
    Fn(FnType),
    Unknown,
    Never,
}

impl Type {
    pub fn make_union(types: impl IntoIterator<Item = Type>) -> Type {
        let mut flat_set = BTreeSet::new();

        for ty in types {
            if matches!(ty, Type::Never) {
                continue;
            }

            if let Type::Union(variants) = ty {
                flat_set.extend(variants);
                continue;
            }

            flat_set.insert(ty);
        }

        match flat_set.len() {
            0 => Type::Never,
            1 => flat_set.into_iter().next().unwrap(),
            _ => Type::Union(flat_set),
        }
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

        if let Type::Union(variants) = self {
            return variants;
        }

        BTreeSet::from([self])
    }

    pub fn get_union_variants(&self) -> Option<&BTreeSet<Type>> {
        match self {
            Type::Union(variants) => Some(variants),
            _ => None,
        }
    }

    /// Maps a struct field name -> (Index, Type)
    pub fn get_field(&self, name: &StringId) -> Option<(usize, Type)> {
        match self {
            Type::Struct(fields) => fields
                .iter()
                .enumerate()
                .find(|(_, param)| &param.identifier.name == name)
                .map(|(index, param)| (index, param.ty.clone())),
            _ => None,
        }
    }
}
