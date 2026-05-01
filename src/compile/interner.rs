use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::RwLock,
};

use crate::{
    globals::STRING_INTERNER,
    hir::types::{
        checked_type::{CheckedParam, FnTypeKind, StructKind, Type},
        ordered_float::{OrderedF32, OrderedF64},
    },
    tokenize::{NumberKind, TokenKind},
};

pub type GenericSubstitutions = HashMap<StringId, TypeId>;

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
    pub fn bool(&self, literal: Option<bool>) -> TypeId {
        self.intern(&Type::Bool(literal))
    }

    pub fn i8(&self, literal: Option<i8>) -> TypeId {
        self.intern(&Type::I8(literal))
    }

    pub fn i16(&self, literal: Option<i16>) -> TypeId {
        self.intern(&Type::I16(literal))
    }

    pub fn i32(&self, literal: Option<i32>) -> TypeId {
        self.intern(&Type::I32(literal))
    }

    pub fn i64(&self, literal: Option<i64>) -> TypeId {
        self.intern(&Type::I64(literal))
    }

    pub fn isize(&self, literal: Option<isize>) -> TypeId {
        self.intern(&Type::ISize(literal))
    }

    pub fn u8(&self, literal: Option<u8>) -> TypeId {
        self.intern(&Type::U8(literal))
    }

    pub fn u16(&self, literal: Option<u16>) -> TypeId {
        self.intern(&Type::U16(literal))
    }

    pub fn u32(&self, literal: Option<u32>) -> TypeId {
        self.intern(&Type::U32(literal))
    }

    pub fn u64(&self, literal: Option<u64>) -> TypeId {
        self.intern(&Type::U64(literal))
    }

    pub fn usize(&self, literal: Option<usize>) -> TypeId {
        self.intern(&Type::USize(literal))
    }

    pub fn ptr(&self, to: TypeId) -> TypeId {
        self.intern(&Type::Pointer(to))
    }

    pub fn f32(&self, literal: Option<OrderedF32>) -> TypeId {
        self.intern(&Type::F32(literal))
    }

    pub fn f64(&self, literal: Option<OrderedF64>) -> TypeId {
        self.intern(&Type::F64(literal))
    }

    pub fn string(&self, literal: Option<StringId>) -> TypeId {
        let header_ty = self.intern(&Type::Struct(StructKind::String(literal)));
        self.ptr(header_ty)
    }

    pub fn unknown(&self) -> TypeId {
        self.intern(&Type::Unknown)
    }

    pub fn null(&self) -> TypeId {
        self.intern(&Type::Null)
    }

    pub fn void(&self) -> TypeId {
        self.intern(&Type::Void)
    }

    pub fn never(&self) -> TypeId {
        self.intern(&Type::Never)
    }
}

impl TypeInterner {
    pub fn unwrap_generic_bound(&self, ty: TypeId) -> TypeId {
        match self.resolve(ty) {
            Type::GenericParam {
                extends: Some(bound),
                ..
            } => self.unwrap_generic_bound(bound),
            _ => ty,
        }
    }

    pub fn unwrap_ptr(&self, ptr: TypeId) -> TypeId {
        if let Type::Pointer(to) = self.resolve(ptr) {
            return to;
        }

        panic!("INTERNAL COMPILER ERROR: Called unwrap_ptr on non-pointer type")
    }

    pub fn from_number_kind(&self, val: &NumberKind) -> TypeId {
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

    pub fn make_union(&self, types: impl IntoIterator<Item = TypeId>) -> TypeId {
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

    pub fn union(&self, a: TypeId, b: TypeId) -> TypeId {
        self.make_union(vec![a, b])
    }

    pub fn union_intersect(&self, a: TypeId, b: TypeId) -> TypeId {
        let s1 = self.as_union_set(a);
        let s2 = self.as_union_set(b);

        let result_types = s1.intersection(&s2).copied().collect::<Vec<_>>();

        self.make_union(result_types)
    }

    pub fn union_subtract(&self, a: TypeId, b: TypeId) -> TypeId {
        let mut s1 = self.as_union_set(a);
        let s2 = self.as_union_set(b);

        for t in s2 {
            s1.remove(&t);
        }

        self.make_union(s1)
    }

    fn as_union_set(&self, ty: TypeId) -> BTreeSet<TypeId> {
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

    pub fn get_numeric_type_rank(&self, id: TypeId) -> Option<i32> {
        use Type::*;
        let ty = self.resolve(id);

        match ty {
            I8(_) | U8(_) => Some(1),
            I16(_) | U16(_) => Some(2),
            I32(_) | U32(_) | ISize(_) | USize(_) => Some(3),
            I64(_) | U64(_) => Some(4),
            F32(_) => Some(5),
            F64(_) => Some(6),
            _ => None,
        }
    }

    pub fn is_float(&self, id: TypeId) -> bool {
        use Type::*;
        let ty = self.resolve(id);

        matches!(ty, F32(_) | F64(_))
    }

    pub fn is_integer(&self, id: TypeId) -> bool {
        use Type::*;
        let ty = self.resolve(id);

        matches!(
            ty,
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

    pub fn is_signed(&self, id: TypeId) -> bool {
        use Type::*;
        let ty = self.resolve(id);
        matches!(
            ty,
            I8(_) | I16(_) | I32(_) | I64(_) | ISize(_) | F32(_) | F64(_)
        )
    }

    pub fn is_pointer(&self, id: TypeId) -> bool {
        matches!(self.resolve(id), Type::Pointer(_))
    }

    pub fn is_bool(&self, id: TypeId) -> bool {
        let ty = self.resolve(id);
        matches!(ty, Type::Bool(_))
    }
}

impl TypeInterner {
    pub fn to_string(&self, target: TypeId) -> String {
        let mut visited_set = HashSet::new();
        self.to_string_recursive(target, &mut visited_set)
    }

    pub fn to_string_recursive(
        &self,
        target: TypeId,
        visited_set: &mut HashSet<TypeId>,
    ) -> String {
        if !visited_set.insert(target) {
            return "...".to_string();
        }

        let result = match self.resolve(target) {
            Type::Bool(lit) => lit.map_or("bool", |v| &v.to_string()).to_string(),
            Type::U8(lit) => format!("{}{}", lit.map_or("", |v| v), "u8"),
            Type::U16(lit) => format!("{}{}", lit.map_or("", |v| v), "u16"),
            Type::U32(lit) => format!("{}{}", lit.map_or("", |v| v), "u32"),
            Type::U64(lit) => format!("{}{}", lit.map_or("", |v| v), "u64"),
            Type::USize(lit) => {
                format!("{}{}", lit.map_or("", |v| &v.to_string()), "usize")
            }
            Type::ISize(lit) => {
                format!("{}{}", lit.map_or("", |v| &v.to_string()), "isize")
            }
            Type::I8(lit) => format!("{}{}", lit.map_or("", |v| &v.to_string()), "i8"),
            Type::I16(lit) => format!("{}{}", lit.map_or("", |v| &v.to_string()), "i16"),
            Type::I32(lit) => format!("{}{}", lit.map_or("", |v| &v.to_string()), "i32"),
            Type::I64(lit) => format!("{}{}", lit.map_or("", |v| &v.to_string()), "i64"),
            Type::F32(lit) => {
                format!("{}{}", lit.map_or("", |v| &v.0.to_string()), "f32")
            }
            Type::F64(lit) => {
                format!("{}{}", lit.map_or("", |v| &v.0.to_string()), "f64")
            }
            Type::Struct(s) => match s {
                StructKind::UserDefined(checked_params) => {
                    self.struct_to_string(&checked_params, visited_set)
                }
                StructKind::TaggedUnion(variants) => {
                    self.union_variants_to_string(&variants, visited_set, true)
                }
                StructKind::ListHeader(item_type) => {
                    self.list_to_string(item_type, visited_set)
                }
                StructKind::String(lit) => String::from("string"),
            },
            Type::Fn(fn_type) => self.fn_signature_to_string(fn_type, visited_set),
            Type::Pointer(to) => {
                format!("ptr<{}>", self.to_string_recursive(to, visited_set))
            }
            Type::TaglessUnion(variants) => {
                self.union_variants_to_string(&variants, visited_set, false)
            }
            Type::GenericParam {
                identifier,
                extends,
            } => {
                let name_str = STRING_INTERNER.resolve(identifier.name);
                if let Some(c) = extends {
                    format!(
                        "{} extends {}",
                        name_str,
                        self.to_string_recursive(c, visited_set)
                    )
                } else {
                    name_str
                }
            }
            Type::Void => String::from("void"),
            Type::Never => String::from("never"),
            Type::Unknown => String::from("unknown"),
            Type::Null => String::from("null"),
        };

        visited_set.remove(&target);

        result
    }

    pub fn token_kind_to_string(&self, kind: &TokenKind) -> String {
        match kind {
            TokenKind::Identifier(id) => STRING_INTERNER.resolve(*id).to_string(),
            TokenKind::Punctuation(punctuation_kind) => punctuation_kind.to_string(),
            TokenKind::Keyword(keyword_kind) => keyword_kind.to_string(),
            TokenKind::String(value) => value.to_owned(),
            TokenKind::Number(number_kind) => number_kind.to_string(),
            TokenKind::Doc(value) => format!("---\n{}\n---", value),
            TokenKind::TemplateString(value) => value.to_owned(),
        }
    }

    pub fn struct_to_string(
        &self,
        fields: &[CheckedParam],
        visited_set: &mut HashSet<TypeId>,
    ) -> String {
        let fields_str = fields
            .iter()
            .map(|f| {
                format!(
                    "{}: {}",
                    STRING_INTERNER.resolve(f.identifier.name),
                    self.to_string_recursive(f.ty.id, visited_set)
                )
            })
            .collect::<Vec<String>>()
            .join(", ");

        format!("{{ {} }}", fields_str)
    }

    fn union_variants_to_string(
        &self,
        variants: &BTreeSet<TypeId>,
        visited_set: &mut HashSet<TypeId>,
        is_tagged: bool,
    ) -> String {
        let symbol = if is_tagged { "|" } else { "~" };

        variants
            .iter()
            .map(|tag| self.to_string_recursive(*tag, visited_set))
            .collect::<Vec<String>>()
            .join(symbol)
    }

    pub fn list_to_string(
        &self,
        item_type: TypeId,
        visited_set: &mut HashSet<TypeId>,
    ) -> String {
        let item_type_string = self.to_string_recursive(item_type, visited_set);
        format!("{}[]", item_type_string)
    }

    fn fn_signature_to_string(
        &self,
        fn_type_kind: FnTypeKind,
        visited_set: &mut HashSet<TypeId>,
    ) -> String {
        match fn_type_kind {
            FnTypeKind::Direct(declaration_id) => {
                format!("fn_{}", declaration_id.0)
            }
            FnTypeKind::Indirect(signature) => {
                let params_str = signature
                    .params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {}",
                            STRING_INTERNER.resolve(p.identifier.name),
                            self.to_string_recursive(p.ty.id, visited_set)
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                let return_type_str =
                    self.to_string_recursive(signature.return_type.id, visited_set);

                format!("fn({}): {}", params_str, return_type_str)
            }
        }
    }
}
