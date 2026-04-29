use crate::{
    ast::{expr::Expr, Span},
    compile::interner::GenericSubstitutions,
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_declaration::CheckedDeclaration,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_return_stmt(
        &mut self,
        value: Expr,
        span: Span,
        substitutions: &GenericSubstitutions,
    ) {
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

        let val_id = self.build_expr(value, Some(&expected_return_type), substitutions);

        self.emit_return(val_id);
    }
}
