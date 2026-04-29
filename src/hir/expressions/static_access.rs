use crate::{
    ast::{expr::Expr, IdentifierNode},
    compile::interner::GenericSubstitutions,
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_static_access_expr(
        &mut self,
        left: Expr,
        field: IdentifierNode,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let span = field.span.clone();

        let left_id = self.build_expr(left, None, substitutions);
        let left_type = self.get_value_type(left_id);

        let result = self.report_error_and_get_poison(SemanticError {
            kind: SemanticErrorKind::CannotStaticAccess(left_type),
            span: span.clone(),
        });

        self.check_expected(result, span, expected_type)
    }
}
