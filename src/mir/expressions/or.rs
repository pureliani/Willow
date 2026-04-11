use crate::{
    ast::expr::Expr,
    compile::interner::GenericSubstitutions,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_or_expr(
        &mut self,
        left: Expr,
        right: Expr,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let span = left.span.clone();
        let bool_type = self.types.bool(None);
        let expected_left = SpannedType {
            id: bool_type,
            span: left.span.clone(),
        };

        let expected_right = SpannedType {
            id: bool_type,
            span: right.span.clone(),
        };

        let left_span = left.span.clone();
        let left_id = self.build_expr(left, Some(&expected_left), substitutions);

        let result = self.emit_logical_or(left_id, left_span, |builder| {
            builder.build_expr(right, Some(&expected_right), substitutions)
        });

        self.check_expected(result, span, expected_type)
    }
}
