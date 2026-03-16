use crate::{
    ast::expr::Expr,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{SpannedType, Type},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_and_expr(
        &mut self,
        left: Expr,
        right: Expr,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let span = left.span.clone();
        let expected_left = SpannedType {
            kind: Type::Bool(None),
            span: left.span.clone(),
        };

        let expected_right = SpannedType {
            kind: Type::Bool(None),
            span: right.span.clone(),
        };

        let left_span = left.span.clone();
        let left_id = self.build_expr(left, Some(&expected_left));

        let result = self.emit_logical_and(left_id, left_span, |builder| {
            builder.build_expr(right, Some(&expected_right))
        });

        self.check_expected(result, span, expected_type)
    }
}
