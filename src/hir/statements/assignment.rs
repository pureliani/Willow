use crate::{
    ast::expr::Expr,
    compile::interner::GenericSubstitutions,
    hir::{
        builders::{Builder, InBlock},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_assignment_stmt(
        &mut self,
        target: Expr,
        value: Expr,
        substitutions: &GenericSubstitutions,
    ) {
        let target_place = match self.resolve_place(target, substitutions) {
            Ok(p) => p,
            Err(e) => {
                self.errors.push(e);
                return;
            }
        };

        let constraint = self.type_of_place(&target_place);
        let value_span = value.span.clone();
        let value_id = self.build_expr(
            value,
            Some(&SpannedType {
                id: constraint,
                span: value_span,
            }),
            substitutions,
        );

        self.write_place(&target_place, value_id);
    }
}
