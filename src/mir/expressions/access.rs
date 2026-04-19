use crate::{
    ast::{
        expr::{Expr, ExprKind},
        IdentifierNode,
    },
    compile::interner::GenericSubstitutions,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_access_expr(
        &mut self,
        left: Expr,
        field: IdentifierNode,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let field_span = field.span.clone();

        let access_expr = Expr {
            kind: ExprKind::Access {
                left: Box::new(left),
                field: field.clone(),
            },
            span: field_span.clone(),
        };

        let place = match self.resolve_place(access_expr, substitutions) {
            Ok(p) => p,
            Err(e) => return self.report_error_and_get_poison(e),
        };

        let field_val = self.read_place(&place);
        self.check_expected(field_val, field_span, expected_type)
    }
}
