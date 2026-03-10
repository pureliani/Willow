use crate::{
    globals::STRING_INTERNER,
    hir::types::{checked_declaration::CheckedParam, checked_type::Type},
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

// Assuming 64-bit architecture
const PTR_SIZE: usize = 8;
const PTR_ALIGN: usize = 8;

pub fn get_layout_of(ty: &Type) -> Layout {
    match ty {
        Type::Void | Type::Never | Type::Unknown | Type::Literal(_) | Type::Null => {
            Layout::new(0, 1)
        }

        Type::Bool | Type::U8 | Type::I8 => Layout::new(1, 1),
        Type::U16 | Type::I16 => Layout::new(2, 2),
        Type::U32 | Type::I32 | Type::F32 => Layout::new(4, 4),

        Type::U64 | Type::I64 | Type::F64 | Type::USize | Type::ISize => {
            Layout::new(8, 8)
        }

        Type::String => Layout::new(PTR_SIZE * 2, PTR_ALIGN), // (ptr, len)
        Type::List(_) => Layout::new(PTR_SIZE * 3, PTR_ALIGN), // (ptr, len, cap)
        Type::Fn(_) => Layout::new(PTR_SIZE, PTR_ALIGN),      // function pointer

        Type::Struct(fields) => {
            let field_types: Vec<&Type> = fields.iter().map(|p| &p.ty.kind).collect();
            calculate_fields_layout(&field_types)
        }

        Type::Union { base, .. } => {
            let mut max_size = 0;
            let mut max_align = 1;

            for v in base {
                let layout = get_layout_of(v);
                max_size = max_size.max(layout.size);
                max_align = max_align.max(layout.alignment);
            }

            let discriminant_size = 2;
            let discriminant_align = 2;

            let total_align = max_align.max(discriminant_align);

            let padding_before_payload =
                (max_align - (discriminant_size % max_align)) % max_align;

            let raw_size = discriminant_size + padding_before_payload + max_size;

            let padding_end = (total_align - (raw_size % total_align)) % total_align;

            let total_size = raw_size + padding_end;

            Layout::new(total_size, total_align)
        }
    }
}

pub fn get_alignment_of(ty: &Type) -> usize {
    get_layout_of(ty).alignment
}

fn calculate_fields_layout(field_types: &[&Type]) -> Layout {
    let mut current_offset = 0;
    let mut max_alignment = 1;

    for ty in field_types {
        let field_layout = get_layout_of(ty);

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

pub fn pack_struct(mut fields: Vec<CheckedParam>) -> Vec<CheckedParam> {
    fields.sort_by(|field_a, field_b| {
        let align_a = get_alignment_of(&field_a.ty.kind);
        let align_b = get_alignment_of(&field_b.ty.kind);

        align_b.cmp(&align_a).then_with(|| {
            let name_a = STRING_INTERNER.resolve(field_a.identifier.name);
            let name_b = STRING_INTERNER.resolve(field_b.identifier.name);
            name_a.cmp(&name_b)
        })
    });

    fields
}
