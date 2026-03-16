use crate::{
    ast::{expr::Expr, Span},
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_binary_op<F>(
        &mut self,
        left: Expr,
        right: Expr,
        op: F,
        expected_type: Option<&SpannedType>,
    ) -> ValueId
    where
        F: FnOnce(&mut Self, ValueId, ValueId) -> ValueId,
    {
        let left_span = left.span.clone();
        let left_value = self.build_expr(left, None);

        let right_span = right.span.clone();
        let right_value = self.build_expr(right, None);

        // TODO: do adjustment before calling op()

        let result = op(self, left_value, right_value);

        let span = Span {
            start: left_span.start,
            end: right_span.end,
            path: left_span.path,
        };

        self.check_expected(result, span, expected_type)
    }
}
