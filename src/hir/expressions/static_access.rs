use crate::{
    ast::{expr::Expr, IdentifierNode},
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        instructions::InstrId,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_static_access_expr(
        &mut self,
        left: Expr,
        field: IdentifierNode,
    ) -> InstrId {
        let span = field.span.clone();

        let result = self.report_error_and_get_poison(SemanticError {
            kind: SemanticErrorKind::CannotStaticAccess(left_type),
            span: span.clone(),
        });

        result
    }
}
