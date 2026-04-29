use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_number_expr(
        &mut self,
        span: Span,
        value: NumberKind,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let result = self.emit_number(value);
        self.check_expected(result, span, expected_type)
    }
}
