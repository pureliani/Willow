use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_bool_expr(
        &mut self,
        span: Span,
        value: bool,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let result = self.emit_bool(value);
        self.check_expected(result, span, expected_type)
    }
}
