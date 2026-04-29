use crate::{
    ast::StringNode,
    globals::STRING_INTERNER,
    hir::{
        builders::{Builder, InBlock},
        instructions::InstrId,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_string_literal(&mut self, node: StringNode) -> InstrId {
        let span = node.span.clone();
        let string_id = STRING_INTERNER.intern(&node.value);
        self.emit_string(string_id)
    }
}
