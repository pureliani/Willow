use crate::{
    ast::{expr::Expr, type_annotation::TypeAnnotation},
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_type::SpannedType,
        utils::{
            adjustment::compute_type_adjustment,
            check_type::{check_type_annotation, TypeCheckerContext},
        },
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
        let source_type = self.get_value_type(source).clone();

        let mut type_ctx = TypeCheckerContext {
            scope: self.current_scope.clone(),
            declarations: &self.program.declarations,
            errors: self.errors,
        };
        let target_type = check_type_annotation(&mut type_ctx, &target);

        let adjusted_val =
            match compute_type_adjustment(&source_type, &target_type.kind, true) {
                Err(_) => self.report_error_and_get_poison(SemanticError {
                    kind: SemanticErrorKind::CannotCastType {
                        source_type,
                        target_type: target_type.kind.clone(),
                    },
                    span: source_span.clone(),
                }),
                Ok(adj) => self.apply_adjustment(source, adj, target_type.kind),
            };

        self.check_expected(adjusted_val, source_span, expected_type)
    }
}
