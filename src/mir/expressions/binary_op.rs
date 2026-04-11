use crate::{
    ast::{expr::Expr, Span},
    compile::interner::GenericSubstitutions,
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::SemanticError,
        types::checked_type::{SpannedType, Type},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_binary_op<F>(
        &mut self,
        left: Expr,
        right: Expr,
        op: F,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId
    where
        F: FnOnce(&mut Self, ValueId, ValueId) -> ValueId,
    {
        let left_span = left.span.clone();
        let left_value = self.build_expr(left, None, substitutions);
        let left_type = self.get_value_type(left_value);

        let right_span = right.span.clone();
        let right_value = self.build_expr(right, None, substitutions);
        let right_type = self.get_value_type(right_value);

        let supertype = match self.arithmetic_supertype(
            left_type,
            left_span.clone(),
            right_type,
            right_span.clone(),
        ) {
            Ok(ty) => {
                if let Type::Literal(li) = self.types.resolve(ty) {
                    self.types.widen_literal(li)
                } else {
                    ty
                }
            }
            Err(e) => return self.report_error_and_get_poison(e),
        };

        let left_adj = match self.compute_adjustment(left_value, supertype, false) {
            Ok(adj) => {
                self.apply_adjustment(left_value, adj, supertype, left_span.clone())
            }
            Err(kind) => {
                return self.report_error_and_get_poison(SemanticError {
                    kind,
                    span: left_span.clone(),
                })
            }
        };

        let right_adj = match self.compute_adjustment(right_value, supertype, false) {
            Ok(adj) => {
                self.apply_adjustment(right_value, adj, supertype, right_span.clone())
            }
            Err(kind) => {
                return self.report_error_and_get_poison(SemanticError {
                    kind,
                    span: right_span.clone(),
                })
            }
        };

        let result = op(self, left_adj, right_adj);

        let span = Span {
            start: left_span.start,
            end: right_span.end,
            path: left_span.path,
        };

        self.check_expected(result, span, expected_type)
    }
}
