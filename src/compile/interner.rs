use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::RwLock,
};

use crate::{
    globals::STRING_INTERNER,
    mir::types::{
        checked_declaration::{CheckedParam, FnType},
        checked_type::{LiteralType, StructKind, Type},
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
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::Bool(value)))
        } else {
            self.intern(&Type::Bool)
        }
    }

    pub fn i8(&self, literal: Option<i8>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::I8(value)))
        } else {
            self.intern(&Type::I8)
        }
    }

    pub fn i16(&self, literal: Option<i16>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::I16(value)))
        } else {
            self.intern(&Type::I16)
        }
    }

    pub fn i32(&self, literal: Option<i32>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::I32(value)))
        } else {
            self.intern(&Type::I32)
        }
    }

    pub fn i64(&self, literal: Option<i64>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::I64(value)))
        } else {
            self.intern(&Type::I64)
        }
    }

    pub fn isize(&self, literal: Option<isize>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::ISize(value)))
        } else {
            self.intern(&Type::ISize)
        }
    }

    pub fn u8(&self, literal: Option<u8>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::U8(value)))
        } else {
            self.intern(&Type::U8)
        }
    }

    pub fn u16(&self, literal: Option<u16>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::U16(value)))
        } else {
            self.intern(&Type::U16)
        }
    }

    pub fn u32(&self, literal: Option<u32>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::U32(value)))
        } else {
            self.intern(&Type::U32)
        }
    }

    pub fn u64(&self, literal: Option<u64>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::U64(value)))
        } else {
            self.intern(&Type::U64)
        }
    }

    pub fn usize(&self, literal: Option<usize>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::USize(value)))
        } else {
            self.intern(&Type::USize)
        }
    }

    pub fn ptr(&self, to: TypeId) -> TypeId {
        self.intern(&Type::Pointer(to))
    }

    pub fn f32(&self, literal: Option<OrderedF32>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::F32(value)))
        } else {
            self.intern(&Type::F32)
        }
    }

    pub fn f64(&self, literal: Option<OrderedF64>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::F64(value)))
        } else {
            self.intern(&Type::F64)
        }
    }

    pub fn string(&self, literal: Option<StringId>) -> TypeId {
        if let Some(value) = literal {
            self.intern(&Type::Literal(LiteralType::String(value)))
        } else {
            let header_ty = self.intern(&Type::Struct(StructKind::String));
            self.ptr(header_ty)
        }
    }

    pub fn unknown(&self) -> TypeId {
        self.intern(&Type::Literal(LiteralType::Unknown))
    }

    pub fn null(&self) -> TypeId {
        self.intern(&Type::Literal(LiteralType::Null))
    }

    pub fn void(&self) -> TypeId {
        self.intern(&Type::Literal(LiteralType::Void))
    }

    pub fn never(&self) -> TypeId {
        self.intern(&Type::Literal(LiteralType::Never))
    }
}

impl TypeInterner {
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

            if matches!(ty, Type::Literal(LiteralType::Never)) {
                continue;
            }

            if let Type::Struct(StructKind::TaggedUnion(variants)) = ty {
                flat.extend(variants);
            } else {
                flat.insert(ty_id);
            }
        }

        if flat.is_empty() {
            return self.intern(&Type::Literal(LiteralType::Never));
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

    pub fn widen_literal(&self, ty: LiteralType) -> TypeId {
        match ty {
            LiteralType::I8(_) => self.i8(None),
            LiteralType::I16(_) => self.i16(None),
            LiteralType::I32(_) => self.i32(None),
            LiteralType::I64(_) => self.i64(None),
            LiteralType::ISize(_) => self.isize(None),

            LiteralType::U8(_) => self.u8(None),
            LiteralType::U16(_) => self.u16(None),
            LiteralType::U32(_) => self.u32(None),
            LiteralType::U64(_) => self.u64(None),
            LiteralType::USize(_) => self.usize(None),

            LiteralType::F32(_) => self.f32(None),
            LiteralType::F64(_) => self.f64(None),

            LiteralType::Bool(_) => self.bool(None),

            LiteralType::String(_) => {
                let target = self.intern(&Type::Struct(StructKind::String));
                self.ptr(target)
            }
            LiteralType::Void => self.void(),
            LiteralType::Never => self.never(),
            LiteralType::Unknown => self.unknown(),
            LiteralType::Null => self.null(),
            LiteralType::Fn(declaration_id) => {
                self.intern(&Type::Literal(LiteralType::Fn(declaration_id)))
            }
        }
    }

    pub fn get_numeric_type_rank(&self, ty: TypeId) -> Option<i32> {
        use Type::*;
        let widened = match self.resolve(ty) {
            Literal(lt) => {
                let widened_id = self.widen_literal(lt);
                self.resolve(widened_id)
            }
            t => t,
        };

        match widened {
            I8 | U8 => Some(1),
            I16 | U16 => Some(2),
            I32 | U32 | ISize | USize => Some(3),
            I64 | U64 => Some(4),
            F32 => Some(5),
            F64 => Some(6),
            _ => None,
        }
    }

    pub fn is_float(&self, ty: TypeId) -> bool {
        use Type::*;
        let widened = match self.resolve(ty) {
            Literal(lt) => {
                let widened_id = self.widen_literal(lt);
                self.resolve(widened_id)
            }
            t => t,
        };

        matches!(widened, F32 | F64)
    }

    pub fn is_integer(&self, ty: TypeId) -> bool {
        use Type::*;
        let widened = match self.resolve(ty) {
            Literal(lt) => {
                let widened_id = self.widen_literal(lt);
                self.resolve(widened_id)
            }
            t => t,
        };
        matches!(
            widened,
            I8 | I16 | I32 | I64 | U8 | U16 | U32 | U64 | ISize | USize
        )
    }

    pub fn is_signed(&self, ty: TypeId) -> bool {
        use Type::*;
        let widened = match self.resolve(ty) {
            Literal(lt) => {
                let widened_id = self.widen_literal(lt);
                self.resolve(widened_id)
            }
            t => t,
        };
        matches!(widened, I8 | I16 | I32 | I64 | ISize | F32 | F64)
    }

    pub fn is_pointer(&self, ty: TypeId) -> bool {
        matches!(self.resolve(ty), Type::Pointer(_))
    }

    pub fn is_bool(&self, ty: TypeId) -> bool {
        let widened = match self.resolve(ty) {
            Type::Literal(lt) => {
                let widened_id = self.widen_literal(lt);
                self.resolve(widened_id)
            }
            t => t,
        };
        matches!(widened, Type::Bool)
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
            Type::Bool => String::from("bool"),
            Type::U8 => String::from("u8"),
            Type::U16 => String::from("u16"),
            Type::U32 => String::from("u32"),
            Type::U64 => String::from("u64"),
            Type::USize => String::from("usize"),
            Type::ISize => String::from("isize"),
            Type::I8 => String::from("i8"),
            Type::I16 => String::from("i16"),
            Type::I32 => String::from("i32"),
            Type::I64 => String::from("i64"),
            Type::F32 => String::from("f32"),
            Type::F64 => String::from("f64"),
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
                StructKind::String => String::from("string"),
            },
            Type::Literal(lit) => match lit {
                LiteralType::Void => String::from("void"),
                LiteralType::Null => String::from("null"),
                LiteralType::Unknown => String::from("unknown"),
                LiteralType::Never => String::from("never"),

                LiteralType::Bool(value) => value.to_string(),
                LiteralType::U8(value) => value.to_string(),
                LiteralType::U16(value) => value.to_string(),
                LiteralType::U32(value) => value.to_string(),
                LiteralType::U64(value) => value.to_string(),
                LiteralType::USize(value) => value.to_string(),
                LiteralType::ISize(value) => value.to_string(),
                LiteralType::I8(value) => value.to_string(),
                LiteralType::I16(value) => value.to_string(),
                LiteralType::I32(value) => value.to_string(),
                LiteralType::I64(value) => value.to_string(),
                LiteralType::F32(value) => value.0.to_string(),
                LiteralType::F64(value) => value.0.to_string(),
                LiteralType::String(string_id) => {
                    format!("\"{}\"", STRING_INTERNER.resolve(string_id))
                }
                LiteralType::Fn(declaration_id) => format!("fn{}", declaration_id.0),
            },
            Type::IndirectFn(fn_type) => {
                self.fn_signature_to_string(&fn_type, visited_set)
            }
            Type::Pointer(to) => {
                format!("ptr<{}>", self.to_string_recursive(to, visited_set))
            }
            Type::TaglessUnion(variants) => {
                self.union_variants_to_string(&variants, visited_set, false)
            }
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
        FnType {
            params,
            return_type,
        }: &FnType,
        visited_set: &mut HashSet<TypeId>,
    ) -> String {
        let params_str = params
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

        let return_type_str = self.to_string_recursive(return_type.id, visited_set);

        format!("fn({}): {}", params_str, return_type_str)
    }
}
