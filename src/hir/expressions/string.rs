use crate::{
    ast::StringNode,
    globals::STRING_INTERNER,
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_string_literal(
        &mut self,
        node: StringNode,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let span = node.span.clone();
        let string_id = STRING_INTERNER.intern(&node.value);

        let val = self.emit_string(string_id);
        self.check_expected(val, span, expected_type)
    }
}
