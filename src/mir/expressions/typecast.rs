use crate::{
    ast::{expr::Expr, type_annotation::TypeAnnotation},
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_typecast_expr(
        &mut self,
        left: Expr,
        target: TypeAnnotation,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let source_span = left.span.clone();
        let source = self.build_expr(left, None);
        let source_type = self.get_value_type(source);

        let target_type = self.check_type_annotation(&target);

        let adjusted_val =
            match self.compute_type_adjustment(source_type, target_type.id, true) {
                Err(_) => self.report_error_and_get_poison(SemanticError {
                    kind: SemanticErrorKind::CannotCastType {
                        source_type,
                        target_type: target_type.id,
                    },
                    span: source_span.clone(),
                }),
                Ok(adj) => self.apply_adjustment(
                    source,
                    adj,
                    target_type.id,
                    source_span.clone(),
                ),
            };

        self.check_expected(adjusted_val, source_span, expected_type)
    }
}
