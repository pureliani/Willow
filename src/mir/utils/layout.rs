use crate::{
    compile::interner::TypeInterner,
    globals::STRING_INTERNER,
    mir::types::{
        checked_declaration::{CheckedParam, FnType},
        checked_type::{StructKind, Type},
    },
};

#[derive(Debug, Clone, Copy)]
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
/// before calling this function if you want minimized padding.
pub fn get_layout_of(
    ty: &Type,
    interner: &TypeInterner,
    ptr_size: usize,
    ptr_align: usize,
) -> Layout {
    match ty {
        Type::Void | Type::Null => Layout::new(0, 1),
        Type::Bool(lit) => lit.map_or_else(|| Layout::new(1, 1), |_| Layout::new(0, 1)),
        Type::U8(lit) => lit.map_or_else(|| Layout::new(1, 1), |_| Layout::new(0, 1)),
        Type::I8(lit) => lit.map_or_else(|| Layout::new(1, 1), |_| Layout::new(0, 1)),

        Type::U16(lit) => lit.map_or_else(|| Layout::new(2, 2), |_| Layout::new(0, 1)),
        Type::I16(lit) => lit.map_or_else(|| Layout::new(2, 2), |_| Layout::new(0, 1)),

        Type::U32(lit) => lit.map_or_else(|| Layout::new(4, 4), |_| Layout::new(0, 1)),
        Type::I32(lit) => lit.map_or_else(|| Layout::new(4, 4), |_| Layout::new(0, 1)),
        Type::F32(lit) => lit.map_or_else(|| Layout::new(4, 4), |_| Layout::new(0, 1)),

        Type::U64(lit) => lit.map_or_else(|| Layout::new(8, 8), |_| Layout::new(0, 1)),
        Type::I64(lit) => lit.map_or_else(|| Layout::new(8, 8), |_| Layout::new(0, 1)),
        Type::F64(lit) => lit.map_or_else(|| Layout::new(8, 8), |_| Layout::new(0, 1)),

        Type::USize(lit) => {
            lit.map_or_else(|| Layout::new(ptr_size, ptr_align), |_| Layout::new(0, 1))
        }
        Type::ISize(lit) => {
            lit.map_or_else(|| Layout::new(ptr_size, ptr_align), |_| Layout::new(0, 1))
        }

        Type::Pointer(_) => Layout::new(ptr_size, ptr_align),
        Type::Fn(fntype) => match fntype {
            FnType::Direct(_) => Layout::new(0, 1),
            FnType::Indirect { .. } => Layout::new(ptr_size, ptr_align),
        },
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
        Type::Unknown | Type::Never => {
            panic!("INTERNAL COMPILER ERROR: Cannot get layout of type `unknown` and `never` types")
        }

        Type::Struct(s) => {
            // String literals are ZSTs
            if let StructKind::StringHeader(Some(_)) = s {
                return Layout::new(0, 1);
            }

            let fields = s.fields(interner);
            let types: Vec<Type> = fields
                .into_iter()
                .map(|(_, ty_id)| interner.resolve(ty_id))
                .collect();

            calculate_fields_layout(&types, interner, ptr_size, ptr_align)
        }
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
