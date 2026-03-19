use std::{collections::HashMap, sync::RwLock};

use crate::{globals::TYPE_INTERNER, mir::types::checked_type::Type};

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

impl TypeId {
    pub fn as_type(&self) -> Type {
        TYPE_INTERNER.resolve(*self)
    }
}
