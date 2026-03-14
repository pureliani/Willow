use crate::{
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

const PTR_SIZE: usize = std::mem::size_of::<usize>();
const PTR_ALIGN: usize = std::mem::align_of::<usize>();

/// IMPORTANT: Make sure user-defined structs are packed (via pack_struct)
/// before calling this function if you want minimized padding.
/// returns None for zero-sized types
pub fn get_layout_of(ty: &Type) -> Option<Layout> {
    match ty {
        Type::Void => None,
        Type::Bool(lit) => lit.map_or_else(|| Some(Layout::new(1, 1)), |_| None),
        Type::U8(lit) => lit.map_or_else(|| Some(Layout::new(1, 1)), |_| None),
        Type::I8(lit) => lit.map_or_else(|| Some(Layout::new(1, 1)), |_| None),

        Type::U16(lit) => lit.map_or_else(|| Some(Layout::new(2, 2)), |_| None),
        Type::I16(lit) => lit.map_or_else(|| Some(Layout::new(2, 2)), |_| None),

        Type::U32(lit) => lit.map_or_else(|| Some(Layout::new(4, 4)), |_| None),
        Type::I32(lit) => lit.map_or_else(|| Some(Layout::new(4, 4)), |_| None),
        Type::F32(lit) => lit.map_or_else(|| Some(Layout::new(4, 4)), |_| None),

        Type::U64(lit) => lit.map_or_else(|| Some(Layout::new(8, 8)), |_| None),
        Type::I64(lit) => lit.map_or_else(|| Some(Layout::new(8, 8)), |_| None),
        Type::F64(lit) => lit.map_or_else(|| Some(Layout::new(8, 8)), |_| None),

        Type::USize(lit) => {
            lit.map_or_else(|| Some(Layout::new(PTR_SIZE, PTR_ALIGN)), |_| None)
        }
        Type::ISize(lit) => {
            lit.map_or_else(|| Some(Layout::new(PTR_SIZE, PTR_ALIGN)), |_| None)
        }

        Type::Pointer(_) => Some(Layout::new(PTR_SIZE, PTR_ALIGN)),
        Type::Fn(fntype) => match fntype {
            FnType::Direct(declaration_id) => None,
            FnType::Indirect { .. } => Some(Layout::new(PTR_SIZE, PTR_ALIGN)),
        },
        Type::TaglessUnion(variants) => {
            assert!(variants.len() > 1);

            let mut max_size = 0;
            let mut max_align = 1;

            for v in variants {
                let Some(layout) = get_layout_of(v) else {
                    continue;
                };

                max_size = max_size.max(layout.size);
                max_align = max_align.max(layout.alignment);
            }

            Some(Layout {
                size: max_size,
                alignment: max_align,
            })
        }
        Type::Unknown | Type::Never => {
            panic!("INTERNAL COMPILER ERROR: Cannot get layout of type `unknown` and `never` types")
        }

        Type::Struct(s) => {
            let fields = s.fields();
            let types: Vec<&Type> = fields.iter().map(|(_, ty)| ty).collect();

            calculate_fields_layout(&types)
        }
    }
}
pub fn get_alignment_of(ty: &Type) -> usize {
    get_layout_of(ty).alignment
}

/// Helper to calculate layout of fields placed sequentially in memory,
/// handles padding between fields and at the end of the struct
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

pub fn pack_struct(struct_kind: StructKind) -> StructKind {
    match struct_kind {
        StructKind::UserDefined(mut fields) => {
            sort_fields(&mut fields);
            StructKind::UserDefined(fields)
        }
        other => other,
    }
}

fn sort_fields(fields: &mut [CheckedParam]) {
    fields.sort_by(|field_a, field_b| {
        let align_a = get_alignment_of(&field_a.ty.kind);
        let align_b = get_alignment_of(&field_b.ty.kind);

        align_b.cmp(&align_a).then_with(|| {
            let name_a = STRING_INTERNER.resolve(field_a.identifier.name);
            let name_b = STRING_INTERNER.resolve(field_b.identifier.name);

            name_a.cmp(&name_b)
        })
    });
}
