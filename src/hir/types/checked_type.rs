use crate::{
    ast::{DeclarationId, IdentifierNode, Span},
    compile::interner::{StringId, TypeId},
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
pub struct StructTypeDefinition(pub Vec<CheckedParam>);

impl StructTypeDefinition {
    pub fn fields(&self) -> Vec<(StringId, TypeId)> {
        self.0
            .iter()
            .map(|p| (p.identifier.name, p.ty.id))
            .collect()
    }

    pub fn get_field(&self, name: &StringId) -> Option<(usize, TypeId)> {
        self.fields()
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
    Unknown,
    Null,
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
    Fn(FnTypeKind),
    Pointer(TypeId),
    MutPointer(TypeId),
    Struct(StructTypeDefinition),
    GenericParam {
        identifier: IdentifierNode,
        extends: Option<TypeId>,
    },
    TaglessUnion(BTreeSet<TypeId>),
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
