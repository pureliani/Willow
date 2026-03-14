use std::collections::{BTreeSet, HashSet};

use crate::{
    globals::STRING_INTERNER,
    mir::types::{
        checked_declaration::{CheckedParam, FnType},
        checked_type::{StructKind, Type},
    },
    tokenize::TokenKind,
};

pub fn token_kind_to_string(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Identifier(id) => STRING_INTERNER.resolve(*id).to_string(),
        TokenKind::Punctuation(punctuation_kind) => punctuation_kind.to_string(),
        TokenKind::Keyword(keyword_kind) => keyword_kind.to_string(),
        TokenKind::String(value) => value.to_owned(),
        TokenKind::Number(number_kind) => number_kind.to_string(),
        TokenKind::Doc(value) => format!("---\n{}\n---", value),
    }
}

pub fn type_to_string(ty: &Type) -> String {
    let mut visited_set = HashSet::new();
    type_to_string_recursive(ty, &mut visited_set)
}

pub fn type_to_string_recursive(ty: &Type, visited_set: &mut HashSet<Type>) -> String {
    if !visited_set.insert(ty.clone()) {
        return "...".to_string();
    }

    let result = match ty {
        Type::Void => String::from("void"),
        Type::Null => String::from("null"),
        Type::Unknown => String::from("unknown"),
        Type::Never => String::from("never"),
        Type::Bool(lit) => {
            let suffix = String::from("bool");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::U8(lit) => {
            let suffix = String::from("u8");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::U16(lit) => {
            let suffix = String::from("u16");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::U32(lit) => {
            let suffix = String::from("u32");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::U64(lit) => {
            let suffix = String::from("u64");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::USize(lit) => {
            let suffix = String::from("usize");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::ISize(lit) => {
            let suffix = String::from("isize");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::I8(lit) => {
            let suffix = String::from("i8");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::I16(lit) => {
            let suffix = String::from("i16");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::I32(lit) => {
            let suffix = String::from("i32");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::I64(lit) => {
            let suffix = String::from("i64");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l, suffix))
        }
        Type::F32(lit) => {
            let suffix = String::from("f32");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l.0, suffix))
        }
        Type::F64(lit) => {
            let suffix = String::from("f64");
            lit.map_or(suffix.clone(), |l| format!("{}{}", l.0, suffix))
        }
        Type::Struct(s) => match s {
            StructKind::UserDefined(checked_params) => {
                struct_to_string(checked_params, visited_set)
            }
            StructKind::TaggedUnion { narrowed, .. } => {
                union_variants_to_string(narrowed, visited_set)
            }
            StructKind::ListHeader(item_type) => list_to_string(item_type, visited_set),
            StructKind::StringHeader(string_id) => {
                let string_ty = String::from("string");
                string_id.map_or(string_ty.clone(), |id| {
                    format!("\"{}\"", STRING_INTERNER.resolve(id))
                })
            }
        },
        Type::Fn(fn_type) => fn_signature_to_string(fn_type, visited_set),
        Type::Pointer(_) => todo!(),
        Type::TaglessUnion(btree_set) => todo!(),
    };

    visited_set.remove(ty);

    result
}

fn struct_to_string(fields: &[CheckedParam], visited_set: &mut HashSet<Type>) -> String {
    let fields_str = fields
        .iter()
        .map(|f| {
            format!(
                "{}: {}",
                STRING_INTERNER.resolve(f.identifier.name),
                type_to_string_recursive(&f.ty.kind, visited_set)
            )
        })
        .collect::<Vec<String>>()
        .join(", ");

    format!("{{ {} }}", fields_str)
}

fn union_variants_to_string(
    variants: &BTreeSet<Type>,
    visited_set: &mut HashSet<Type>,
) -> String {
    variants
        .iter()
        .map(|tag| type_to_string_recursive(tag, visited_set))
        .collect::<Vec<String>>()
        .join(" | ")
}

pub fn list_to_string(item_type: &Type, visited_set: &mut HashSet<Type>) -> String {
    let item_type_string = type_to_string_recursive(item_type, visited_set);
    format!("{}[]", item_type_string)
}

fn fn_signature_to_string(fn_type: &FnType, visited_set: &mut HashSet<Type>) -> String {
    match fn_type {
        FnType::Direct(id) => format!("fn_{}", id.0),
        FnType::Indirect {
            params,
            return_type,
        } => {
            let params_str = params
                .iter()
                .map(|p| {
                    format!(
                        "{}: {}",
                        STRING_INTERNER.resolve(p.identifier.name),
                        type_to_string_recursive(&p.ty.kind, visited_set)
                    )
                })
                .collect::<Vec<String>>()
                .join(", ");

            let return_type_str =
                type_to_string_recursive(&return_type.kind, visited_set);

            format!("fn({}): {}", params_str, return_type_str)
        }
    }
}
