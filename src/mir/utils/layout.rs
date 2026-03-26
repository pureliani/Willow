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

const PTR_SIZE: usize = std::mem::size_of::<usize>();
const PTR_ALIGN: usize = std::mem::align_of::<usize>();

/// IMPORTANT: Make sure user-defined structs are packed (via pack_struct)
/// before calling this function if you want minimized padding.
/// returns None for zero-sized types
pub fn get_layout_of(ty: &Type, interner: &TypeInterner) -> Option<Layout> {
    match ty {
        Type::Void | Type::Null => None,
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
            FnType::Direct(_) => None,
            FnType::Indirect { .. } => Some(Layout::new(PTR_SIZE, PTR_ALIGN)),
        },
        Type::TaglessUnion(variants) => {
            assert!(variants.len() > 1);

            let mut max_size = 0;
            let mut max_align = 1;
            let mut has_non_zst = false;

            for v in variants {
                let resolved_v = interner.resolve(*v);
                if let Some(layout) = get_layout_of(&resolved_v, interner) {
                    has_non_zst = true;
                    max_size = max_size.max(layout.size);
                    max_align = max_align.max(layout.alignment);
                }
            }

            if has_non_zst {
                let padding = (max_align - (max_size % max_align)) % max_align;
                Some(Layout {
                    size: max_size + padding,
                    alignment: max_align,
                })
            } else {
                None
            }
        }
        Type::Unknown | Type::Never => {
            panic!("INTERNAL COMPILER ERROR: Cannot get layout of type `unknown` and `never` types")
        }

        Type::Struct(s) => {
            // String literals are ZSTs
            if let StructKind::StringHeader(Some(_)) = s {
                return None;
            }

            let fields = s.fields(interner);
            let types: Vec<Type> =
                fields.into_iter().map(|f| interner.resolve(f.ty)).collect();

            calculate_fields_layout(&types, interner)
        }
    }
}

pub fn get_alignment_of(ty: &Type, interner: &TypeInterner) -> usize {
    get_layout_of(ty, interner)
        .map(|l| l.alignment)
        .unwrap_or(1)
}

/// Helper to calculate layout of fields placed sequentially in memory,
/// handles padding between fields and at the end of the struct
fn calculate_fields_layout(
    field_types: &[Type],
    interner: &TypeInterner,
) -> Option<Layout> {
    let mut current_offset = 0;
    let mut max_alignment = 1;
    let mut has_non_zst = false;

    for ty in field_types {
        if let Some(field_layout) = get_layout_of(ty, interner) {
            has_non_zst = true;
            max_alignment = std::cmp::max(max_alignment, field_layout.alignment);

            let padding = (field_layout.alignment
                - (current_offset % field_layout.alignment))
                % field_layout.alignment;

            current_offset += padding;
            current_offset += field_layout.size;
        }
    }

    if !has_non_zst {
        return None;
    }

    let padding_end = (max_alignment - (current_offset % max_alignment)) % max_alignment;
    let total_size = current_offset + padding_end;

    Some(Layout::new(total_size, max_alignment))
}

pub fn pack_struct(struct_kind: StructKind, interner: &TypeInterner) -> StructKind {
    match struct_kind {
        StructKind::UserDefined(mut fields) => {
            sort_fields(&mut fields, interner);
            StructKind::UserDefined(fields)
        }
        other => other,
    }
}

fn sort_fields(fields: &mut [CheckedParam], interner: &TypeInterner) {
    fields.sort_by(|field_a, field_b| {
        let ty_a = interner.resolve(field_a.ty.id);
        let ty_b = interner.resolve(field_b.ty.id);

        let align_a = get_alignment_of(&ty_a, interner);
        let align_b = get_alignment_of(&ty_b, interner);

        align_b.cmp(&align_a).then_with(|| {
            let name_a = STRING_INTERNER.resolve(field_a.identifier.name);
            let name_b = STRING_INTERNER.resolve(field_b.identifier.name);

            name_a.cmp(&name_b)
        })
    });
}
