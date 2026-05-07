use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::RwLock,
};

use crate::{
    globals::STRING_INTERNER,
    hir::types::checked_type::{CheckedParam, FnTypeKind, Type},
    tokenize::TokenKind,
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
    pub fn bool(&self) -> TypeId {
        self.intern(&Type::Bool)
    }

    pub fn i8(&self) -> TypeId {
        self.intern(&Type::I8)
    }

    pub fn i16(&self) -> TypeId {
        self.intern(&Type::I16)
    }

    pub fn i32(&self) -> TypeId {
        self.intern(&Type::I32)
    }

    pub fn i64(&self) -> TypeId {
        self.intern(&Type::I64)
    }

    pub fn isize(&self) -> TypeId {
        self.intern(&Type::ISize)
    }

    pub fn u8(&self) -> TypeId {
        self.intern(&Type::U8)
    }

    pub fn u16(&self) -> TypeId {
        self.intern(&Type::U16)
    }

    pub fn u32(&self) -> TypeId {
        self.intern(&Type::U32)
    }

    pub fn u64(&self) -> TypeId {
        self.intern(&Type::U64)
    }

    pub fn usize(&self) -> TypeId {
        self.intern(&Type::USize)
    }

    pub fn ptr(&self, to: TypeId) -> TypeId {
        self.intern(&Type::Pointer(to))
    }

    pub fn f32(&self) -> TypeId {
        self.intern(&Type::F32)
    }

    pub fn f64(&self) -> TypeId {
        self.intern(&Type::F64)
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

    pub fn get_numeric_type_rank(&self, id: TypeId) -> Option<i32> {
        use Type::*;
        let ty = self.resolve(id);

        match ty {
            I8 | U8 => Some(1),
            I16 | U16 => Some(2),
            I32 | U32 | ISize | USize => Some(3),
            I64 | U64 => Some(4),
            F32 => Some(5),
            F64 => Some(6),
            _ => None,
        }
    }

    pub fn is_float(&self, id: TypeId) -> bool {
        use Type::*;
        let ty = self.resolve(id);

        matches!(ty, F32 | F64)
    }

    pub fn is_integer(&self, id: TypeId) -> bool {
        use Type::*;
        let ty = self.resolve(id);

        matches!(
            ty,
            I8 | I16 | I32 | I64 | U8 | U16 | U32 | U64 | ISize | USize
        )
    }

    pub fn is_signed(&self, id: TypeId) -> bool {
        use Type::*;
        let ty = self.resolve(id);
        matches!(ty, I8 | I16 | I32 | I64 | ISize | F32 | F64)
    }

    pub fn is_pointer(&self, id: TypeId) -> bool {
        matches!(self.resolve(id), Type::Pointer(_))
    }

    pub fn is_bool(&self, id: TypeId) -> bool {
        let ty = self.resolve(id);
        matches!(ty, Type::Bool)
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
            Type::Bool => "bool".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::USize => "usize".to_string(),
            Type::ISize => "isize".to_string(),
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),

            Type::Struct(s) => self.struct_to_string(&s.0, visited_set),
            Type::Fn(fn_type) => self.fn_signature_to_string(fn_type, visited_set),
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
            Type::Unknown => String::from("unknown"),
            Type::Null => String::from("null"),
            Type::Pointer(to) => {
                format!("*{}", self.to_string_recursive(to, visited_set))
            }
            Type::MutPointer(to) => {
                format!("*mut {}", self.to_string_recursive(to, visited_set))
            }
            Type::TaglessUnion(btree_set) => {
                self.union_variants_to_string(&btree_set, visited_set)
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
    ) -> String {
        variants
            .iter()
            .map(|tag| self.to_string_recursive(*tag, visited_set))
            .collect::<Vec<String>>()
            .join(" or ")
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
