#![allow(clippy::result_large_err)]
#![allow(clippy::inherent_to_string)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::large_enum_variant)]

pub mod ast;
pub mod codegen;
pub mod compile;
pub mod globals;
pub mod hir;
pub mod parse;
pub mod tokenize;
