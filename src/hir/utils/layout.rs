use crate::{
    compile::interner::TypeInterner,
    globals::STRING_INTERNER,
    hir::types::checked_type::{CheckedParam, FnTypeKind, StructKind, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub size: usize,
    pub alignment: usize,
}
impl Layout {
    pub fn new(size: usize, alignment: usize) -> Self {
        Self { size, alignment }
    }
}

/// IMPORTANT: Make sure user-defined structs are packed (via pack_struct)
/// before calling this function if you want minimized padding
pub fn get_layout_of(
    ty: &Type,
    interner: &TypeInterner,
    ptr_size: usize,
    ptr_align: usize,
) -> Layout {
    let zst_layout = Layout::new(0, 1);

    match ty {
        Type::Void => zst_layout,
        Type::Never => zst_layout,
        Type::Unknown => zst_layout,
        Type::Null => zst_layout,
        Type::Bool(lit) => {
            if lit.is_none() {
                Layout::new(1, 1)
            } else {
                zst_layout
            }
        }
        Type::U8(lit) => {
            if lit.is_none() {
                Layout::new(1, 1)
            } else {
                zst_layout
            }
        }
        Type::I8(lit) => {
            if lit.is_none() {
                Layout::new(1, 1)
            } else {
                zst_layout
            }
        }
        Type::U16(lit) => {
            if lit.is_none() {
                Layout::new(2, 2)
            } else {
                zst_layout
            }
        }
        Type::I16(lit) => {
            if lit.is_none() {
                Layout::new(2, 2)
            } else {
                zst_layout
            }
        }
        Type::U32(lit) => {
            if lit.is_none() {
                Layout::new(4, 4)
            } else {
                zst_layout
            }
        }
        Type::I32(lit) => {
            if lit.is_none() {
                Layout::new(4, 4)
            } else {
                zst_layout
            }
        }
        Type::F32(lit) => {
            if lit.is_none() {
                Layout::new(4, 4)
            } else {
                zst_layout
            }
        }
        Type::U64(lit) => {
            if lit.is_none() {
                Layout::new(8, 8)
            } else {
                zst_layout
            }
        }
        Type::I64(lit) => {
            if lit.is_none() {
                Layout::new(8, 8)
            } else {
                zst_layout
            }
        }
        Type::F64(lit) => {
            if lit.is_none() {
                Layout::new(8, 8)
            } else {
                zst_layout
            }
        }
        Type::USize(lit) => {
            if lit.is_none() {
                Layout::new(ptr_size, ptr_align)
            } else {
                zst_layout
            }
        }
        Type::ISize(lit) => {
            if lit.is_none() {
                Layout::new(ptr_size, ptr_align)
            } else {
                zst_layout
            }
        }
        Type::Fn(fn_type_kind) => match fn_type_kind {
            FnTypeKind::Direct(_) => Layout::new(0, 1),
            FnTypeKind::Indirect(_) => Layout::new(ptr_size, ptr_align),
        },
        Type::Pointer(inner_id) => {
            let inner_ty = interner.resolve(*inner_id);
            let inner_layout = get_layout_of(&inner_ty, interner, ptr_size, ptr_align);

            if inner_layout.size == 0 {
                Layout::new(0, 1)
            } else {
                Layout::new(ptr_size, ptr_align)
            }
        }
        Type::TaglessUnion(variants) => {
            assert!(variants.len() > 1);

            let mut max_size = 0;
            let mut max_align = 1;

            for v in variants {
                let resolved_v = interner.resolve(*v);
                let layout = get_layout_of(&resolved_v, interner, ptr_size, ptr_align);

                max_size = max_size.max(layout.size);
                max_align = max_align.max(layout.alignment);
            }

            let padding = (max_align - (max_size % max_align)) % max_align;
            Layout::new(max_size + padding, max_align)
        }
        Type::Struct(s) => {
            let fields = s.fields(interner);
            let types: Vec<Type> = fields
                .into_iter()
                .map(|(_, id)| interner.resolve(id))
                .collect();
            calculate_fields_layout(&types, interner, ptr_size, ptr_align)
        }
        Type::GenericParam { .. } => Layout::new(0, 1),
    }
}

pub fn get_alignment_of(
    ty: &Type,
    interner: &TypeInterner,
    ptr_size: usize,
    ptr_align: usize,
) -> usize {
    get_layout_of(ty, interner, ptr_size, ptr_align).alignment
}

/// Helper to calculate layout of fields placed sequentially in memory,
/// handles padding between fields and at the end of the struct
fn calculate_fields_layout(
    field_types: &[Type],
    interner: &TypeInterner,
    ptr_size: usize,
    ptr_align: usize,
) -> Layout {
    let mut current_offset = 0;
    let mut max_alignment = 1;

    for ty in field_types {
        let field_layout = get_layout_of(ty, interner, ptr_size, ptr_align);
        max_alignment = std::cmp::max(max_alignment, field_layout.alignment);

        let padding = (field_layout.alignment
            - (current_offset % field_layout.alignment))
            % field_layout.alignment;

        current_offset += padding;
        current_offset += field_layout.size;
    }

    let padding_end = (max_alignment - (current_offset % max_alignment)) % max_alignment;
    let total_size = current_offset + padding_end;

    Layout::new(total_size, max_alignment)
}

pub fn pack_struct(
    struct_kind: StructKind,
    interner: &TypeInterner,
    ptr_size: usize,
    ptr_align: usize,
) -> StructKind {
    match struct_kind {
        StructKind::UserDefined(mut fields) => {
            sort_fields(&mut fields, interner, ptr_size, ptr_align);
            StructKind::UserDefined(fields)
        }
        other => other,
    }
}

fn sort_fields(
    fields: &mut [CheckedParam],
    interner: &TypeInterner,
    ptr_size: usize,
    ptr_align: usize,
) {
    fields.sort_by(|field_a, field_b| {
        let ty_a = interner.resolve(field_a.ty.id);
        let ty_b = interner.resolve(field_b.ty.id);

        let align_a = get_alignment_of(&ty_a, interner, ptr_size, ptr_align);
        let align_b = get_alignment_of(&ty_b, interner, ptr_size, ptr_align);

        align_b.cmp(&align_a).then_with(|| {
            let name_a = STRING_INTERNER.resolve(field_a.identifier.name);
            let name_b = STRING_INTERNER.resolve(field_b.identifier.name);

            name_a.cmp(&name_b)
        })
    });
}
