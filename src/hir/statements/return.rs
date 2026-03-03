use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_declaration::CheckedDeclaration,
        utils::check_assignable::compute_type_adjustment,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_return_stmt(&mut self, value: Expr, span: Span) {
        let value_span = value.span.clone();
        let func_id = self.context.func_id;
        let expected_return_type = match self.program.declarations.get(&func_id) {
            Some(CheckedDeclaration::Function(f)) => f.return_type.clone(),
            _ => {
                self.errors.push(SemanticError {
                    kind: SemanticErrorKind::ReturnKeywordOutsideFunction,
                    span: span.clone(),
                });
                return;
            }
        };

        let val_id = self.build_expr(value);
        let actual_type = self.get_value_type(val_id).clone();

        let adjusted_val_id =
            match compute_type_adjustment(&actual_type, &expected_return_type, false) {
                Ok(adj) => self.apply_adjustment(val_id, adj, expected_return_type),
                Err(_) => self.report_error_and_get_poison(SemanticError {
                    kind: SemanticErrorKind::ReturnTypeMismatch {
                        expected: expected_return_type,
                        received: actual_type,
                    },
                    span: value_span,
                }),
            };

        self.emit_return(adjusted_val_id);
    }
}
