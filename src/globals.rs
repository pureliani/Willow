use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;

use crate::ast::DeclarationId;
use crate::compile::interner::{StringId, StringInterner};

pub struct CommonIdentifiers {
    pub ptr: StringId,
    pub cap: StringId,
    pub len: StringId,
    pub id: StringId,
    pub val: StringId,
}

pub static DECLARATION_COUNTER: LazyLock<AtomicUsize> =
    LazyLock::new(|| AtomicUsize::new(0));
pub static GENERIC_DECLARATION_COUNTER: LazyLock<AtomicUsize> =
    LazyLock::new(|| AtomicUsize::new(0));

pub static STRING_INTERNER: LazyLock<StringInterner> =
    LazyLock::new(StringInterner::default);

pub static COMMON_IDENTIFIERS: LazyLock<CommonIdentifiers> =
    LazyLock::new(|| CommonIdentifiers {
        id: STRING_INTERNER.intern("id"),
        val: STRING_INTERNER.intern("val"),
        len: STRING_INTERNER.intern("len"),
        cap: STRING_INTERNER.intern("cap"),
        ptr: STRING_INTERNER.intern("ptr"),
    });

pub fn next_declaration_id() -> DeclarationId {
    DeclarationId(DECLARATION_COUNTER.fetch_add(1, Ordering::SeqCst))
}

pub fn reset_globals() {
    DECLARATION_COUNTER.store(0, Ordering::SeqCst);
    STRING_INTERNER.clear();
}
