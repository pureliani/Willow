use crate::{
    ast::StringNode,
    globals::STRING_INTERNER,
    mir::{
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

        let struct_val = self.emit_string(string_id);
        let struct_ty = self.get_value_type(struct_val);

        let ptr = self.emit_stack_alloc(struct_ty, 1);
        self.emit_store(ptr, struct_val);

        self.check_expected(ptr, span, expected_type)
    }
}
