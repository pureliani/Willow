use std::collections::BTreeSet;

use crate::{
    compile::interner::{StringId, TypeId},
    mir::types::checked_type::{StructKind, Type},
};

fn get_type_at_path(mut ty: TypeId, path: &[StringId]) -> Option<TypeId> {
    for field_name in path {
        match ty.as_type() {
            Type::Struct(StructKind::UserDefined(fields)) => {
                let field = fields.iter().find(|f| f.identifier.name == *field_name)?;
                ty = field.ty.id;
            }
            _ => return None,
        }
    }
    Some(ty)
}

pub fn get_matching_variant_indices(
    union_variants: &BTreeSet<TypeId>,
    path: &[StringId],
    target_type: &TypeId,
) -> Vec<u16> {
    let mut matching_indices = Vec::new();

    for (index, variant) in union_variants.iter().enumerate() {
        let index = index as u16;

        if let Some(field_type) = get_type_at_path(*variant, path) {
            if &field_type == target_type {
                matching_indices.push(index);
            }
        }
    }

    matching_indices
}

pub fn get_non_matching_variant_indices(
    union_variants: &BTreeSet<TypeId>,
    path: &[StringId],
    target_type: &TypeId,
) -> Vec<u16> {
    let matching = get_matching_variant_indices(union_variants, path, target_type);

    let total_variants = union_variants.len();

    let mut non_matching = Vec::with_capacity(total_variants - matching.len());

    let mut match_iter = matching.iter().peekable();

    for i in 0..(total_variants as u16) {
        if match_iter.peek() == Some(&&i) {
            match_iter.next();
        } else {
            non_matching.push(i);
        }
    }

    non_matching
}
