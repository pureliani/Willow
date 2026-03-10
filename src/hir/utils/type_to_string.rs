use std::collections::{BTreeSet, HashSet};

use crate::{
    globals::STRING_INTERNER,
    hir::types::{
        checked_declaration::{CheckedParam, FnType},
        checked_type::{LiteralType, Type},
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
        Type::Bool => String::from("bool"),
        Type::U8 => String::from("u8"),
        Type::U16 => String::from("u16"),
        Type::U32 => String::from("u32"),
        Type::U64 => String::from("u64"),
        Type::USize => String::from("usize"),
        Type::ISize => String::from("isize"),
        Type::I8 => String::from("i8"),
        Type::I16 => String::from("i16"),
        Type::I32 => String::from("i32"),
        Type::I64 => String::from("i64"),
        Type::F32 => String::from("f32"),
        Type::F64 => String::from("f64"),
        Type::String => String::from("string"),
        Type::Null => String::from("null"),
        Type::Unknown => String::from("unknown"),
        Type::Literal(literal) => literal_to_string(literal),
        Type::Never => String::from("never"),
        Type::Struct(s) => struct_to_string(s, visited_set),
        Type::Union { narrowed, .. } => union_variants_to_string(narrowed, visited_set),
        Type::Fn(fn_type) => fn_signature_to_string(fn_type, visited_set),
        Type::List(item_type) => list_to_string(&item_type.kind, visited_set),
    };

    visited_set.remove(ty);

    result
}

pub fn literal_to_string(literal: &LiteralType) -> String {
    match literal {
        LiteralType::Number(ordered_number_kind) => ordered_number_kind.0.to_string(),
        LiteralType::Bool(value) => value.to_string(),
        LiteralType::String(id) => STRING_INTERNER.resolve(*id).to_string(),
    }
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
    let params_str = fn_type
        .params
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
        type_to_string_recursive(&fn_type.return_type.kind, visited_set);

    format!("fn({}): {}", params_str, return_type_str)
}
