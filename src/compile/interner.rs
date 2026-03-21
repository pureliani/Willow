use std::{
    collections::{BTreeSet, HashMap},
    sync::RwLock,
};

use crate::{
    mir::types::{
        checked_type::{StructKind, Type},
        ordered_float::{OrderedF32, OrderedF64},
    },
    tokenize::NumberKind,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StringId(pub usize);

#[derive(Default)]
struct StringInternerState {
    forward: HashMap<String, usize>,
    backward: Vec<String>,
}

#[derive(Default)]
pub struct StringInterner {
    state: RwLock<StringInternerState>,
}

impl StringInterner {
    pub fn intern(&self, key: &str) -> StringId {
        let reader = self.state.read().unwrap();
        if let Some(&index) = reader.forward.get(key) {
            return StringId(index);
        }
        drop(reader);

        let mut writer = self.state.write().unwrap();

        if let Some(&index) = writer.forward.get(key) {
            return StringId(index);
        }

        let index = writer.backward.len();
        writer.backward.push(key.to_owned());
        writer.forward.insert(key.to_owned(), index);

        StringId(index)
    }

    pub fn resolve(&self, key: StringId) -> String {
        let reader = self.state.read().unwrap();
        reader
            .backward
            .get(key.0)
            .unwrap_or_else(|| {
                panic!(
                    "INTERNAL COMPILER ERROR: interner expected key {} to exist",
                    key.0
                )
            })
            .clone()
    }

    pub fn clear(&self) {
        let mut writer = self.state.write().unwrap();
        writer.forward.clear();
        writer.backward.clear();
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u32);

#[derive(Default)]
struct TypeInternerState {
    forward: HashMap<Type, u32>,
    backward: Vec<Type>,
}

#[derive(Default)]
pub struct TypeInterner {
    state: RwLock<TypeInternerState>,
}

impl TypeInterner {
    pub fn intern(&self, ty: &Type) -> TypeId {
        let reader = self.state.read().unwrap();
        if let Some(&index) = reader.forward.get(ty) {
            return TypeId(index);
        }
        drop(reader);

        let mut writer = self.state.write().unwrap();

        if let Some(&index) = writer.forward.get(ty) {
            return TypeId(index);
        }

        let index = writer.backward.len() as u32;
        writer.backward.push(ty.clone());
        writer.forward.insert(ty.clone(), index);

        TypeId(index)
    }

    pub fn resolve(&self, id: TypeId) -> Type {
        let reader = self.state.read().unwrap();
        reader
            .backward
            .get(id.0 as usize)
            .unwrap_or_else(|| {
                panic!(
                    "INTERNAL COMPILER ERROR: interner expected type id {} to exist",
                    id.0
                )
            })
            .clone()
    }

    pub fn clear(&self) {
        let mut writer = self.state.write().unwrap();
        writer.forward.clear();
        writer.backward.clear();
    }
}

impl TypeInterner {
    pub fn bool(&mut self, literal: Option<bool>) -> TypeId {
        self.intern(&Type::Bool(literal))
    }

    pub fn i8(&mut self, literal: Option<i8>) -> TypeId {
        self.intern(&Type::I8(literal))
    }

    pub fn i16(&mut self, literal: Option<i16>) -> TypeId {
        self.intern(&Type::I16(literal))
    }

    pub fn i32(&mut self, literal: Option<i32>) -> TypeId {
        self.intern(&Type::I32(literal))
    }

    pub fn i64(&mut self, literal: Option<i64>) -> TypeId {
        self.intern(&Type::I64(literal))
    }

    pub fn isize(&mut self, literal: Option<isize>) -> TypeId {
        self.intern(&Type::ISize(literal))
    }

    pub fn u8(&mut self, literal: Option<u8>) -> TypeId {
        self.intern(&Type::U8(literal))
    }

    pub fn u16(&mut self, literal: Option<u16>) -> TypeId {
        self.intern(&Type::U16(literal))
    }

    pub fn u32(&mut self, literal: Option<u32>) -> TypeId {
        self.intern(&Type::U32(literal))
    }

    pub fn u64(&mut self, literal: Option<u64>) -> TypeId {
        self.intern(&Type::U64(literal))
    }

    pub fn usize(&mut self, literal: Option<usize>) -> TypeId {
        self.intern(&Type::USize(literal))
    }

    pub fn ptr(&mut self, to: TypeId) -> TypeId {
        self.intern(&Type::Pointer(to))
    }

    pub fn f32(&mut self, literal: Option<OrderedF32>) -> TypeId {
        self.intern(&Type::F32(literal))
    }

    pub fn f64(&mut self, literal: Option<OrderedF64>) -> TypeId {
        self.intern(&Type::F64(literal))
    }

    pub fn unknown(&mut self) -> TypeId {
        self.intern(&Type::Unknown)
    }

    pub fn null(&mut self) -> TypeId {
        self.intern(&Type::Null)
    }

    pub fn void(&mut self) -> TypeId {
        self.intern(&Type::Void)
    }

    pub fn never(&mut self) -> TypeId {
        self.intern(&Type::Never)
    }
}

impl TypeInterner {
    pub fn unwrap_ptr(&self, ptr: TypeId) -> TypeId {
        if let Type::Pointer(to) = self.resolve(ptr) {
            return to;
        }

        panic!("INTERNAL COMPILER ERROR: Called unwrap_ptr on non-pointer type")
    }

    pub fn from_number_kind(&mut self, val: &NumberKind) -> TypeId {
        match *val {
            NumberKind::I8(v) => self.i8(Some(v)),
            NumberKind::I16(v) => self.i16(Some(v)),
            NumberKind::I32(v) => self.i32(Some(v)),
            NumberKind::I64(v) => self.i64(Some(v)),
            NumberKind::ISize(v) => self.isize(Some(v)),
            NumberKind::U8(v) => self.u8(Some(v)),
            NumberKind::U16(v) => self.u16(Some(v)),
            NumberKind::U32(v) => self.u32(Some(v)),
            NumberKind::U64(v) => self.u64(Some(v)),
            NumberKind::USize(v) => self.usize(Some(v)),
            NumberKind::F32(v) => self.f32(Some(OrderedF32(v))),
            NumberKind::F64(v) => self.f64(Some(OrderedF64(v))),
        }
    }

    pub fn make_union(&mut self, types: impl IntoIterator<Item = TypeId>) -> TypeId {
        let mut flat = BTreeSet::new();

        for ty_id in types {
            let ty = self.resolve(ty_id);

            if matches!(ty, Type::Never) {
                continue;
            }

            if let Type::Struct(StructKind::TaggedUnion(variants)) = ty {
                flat.extend(variants);
            } else {
                flat.insert(ty_id);
            }
        }

        if flat.is_empty() {
            return self.intern(&Type::Never);
        }

        if flat.len() == 1 {
            return *flat.iter().next().unwrap();
        }

        self.intern(&Type::Struct(StructKind::TaggedUnion(flat)))
    }

    pub fn union(&mut self, a: TypeId, b: TypeId) -> TypeId {
        self.make_union(vec![a, b])
    }

    pub fn union_intersect(&mut self, a: TypeId, b: TypeId) -> TypeId {
        let s1 = self.as_union_set(a);
        let s2 = self.as_union_set(b);

        let result_types = s1.intersection(&s2).copied().collect::<Vec<_>>();

        self.make_union(result_types)
    }

    pub fn union_subtract(&mut self, a: TypeId, b: TypeId) -> TypeId {
        let mut s1 = self.as_union_set(a);
        let s2 = self.as_union_set(b);

        for t in s2 {
            s1.remove(&t);
        }

        self.make_union(s1)
    }

    fn as_union_set(&mut self, ty: TypeId) -> BTreeSet<TypeId> {
        if ty == self.never() {
            return BTreeSet::new();
        }
        if let Type::Struct(StructKind::TaggedUnion(variants)) = self.resolve(ty) {
            return variants;
        }

        BTreeSet::from([ty])
    }

    pub fn get_union_variants(&self, ty: TypeId) -> Option<BTreeSet<TypeId>> {
        if let Type::Struct(StructKind::TaggedUnion(variants)) = self.resolve(ty) {
            Some(variants.clone())
        } else {
            None
        }
    }

    /// Helper to strip the literal value, returning the generic type.
    /// e.g., I32(Some(5)) -> I32(None)
    pub fn widen_literal(&mut self, ty: TypeId) -> TypeId {
        match self.resolve(ty) {
            Type::I8(_) => self.i8(None),
            Type::I16(_) => self.i16(None),
            Type::I32(_) => self.i32(None),
            Type::I64(_) => self.i64(None),
            Type::ISize(_) => self.isize(None),

            Type::U8(_) => self.u8(None),
            Type::U16(_) => self.u16(None),
            Type::U32(_) => self.u32(None),
            Type::U64(_) => self.u64(None),
            Type::USize(_) => self.usize(None),

            Type::F32(_) => self.f32(None),
            Type::F64(_) => self.f64(None),

            Type::Bool(_) => self.bool(None),

            Type::Struct(StructKind::StringHeader(_)) => {
                self.intern(&Type::Struct(StructKind::StringHeader(None)))
            }

            _ => ty,
        }
    }

    pub fn get_numeric_type_rank(&self, ty: TypeId) -> Option<i32> {
        use Type::*;
        match self.resolve(ty) {
            I8(_) | U8(_) => Some(1),
            I16(_) | U16(_) => Some(2),
            I32(_) | U32(_) | ISize(_) | USize(_) => Some(3),
            I64(_) | U64(_) => Some(4),
            F32(_) => Some(5),
            F64(_) => Some(6),
            _ => None,
        }
    }

    pub fn is_float(&self, ty: TypeId) -> bool {
        use Type::*;
        matches!(self.resolve(ty), F32(_) | F64(_))
    }

    pub fn is_integer(&self, ty: TypeId) -> bool {
        use Type::*;
        matches!(
            self.resolve(ty),
            I8(_)
                | I16(_)
                | I32(_)
                | I64(_)
                | U8(_)
                | U16(_)
                | U32(_)
                | U64(_)
                | ISize(_)
                | USize(_)
        )
    }

    pub fn is_signed(&self, ty: TypeId) -> bool {
        use Type::*;
        matches!(
            self.resolve(ty),
            I8(_) | I16(_) | I32(_) | I64(_) | ISize(_) | F32(_) | F64(_)
        )
    }

    pub fn is_pointer(&self, ty: TypeId) -> bool {
        matches!(self.resolve(ty), Type::Pointer(_))
    }

    pub fn is_bool(&self, ty: TypeId) -> bool {
        matches!(self.resolve(ty), Type::Bool(_))
    }
}
