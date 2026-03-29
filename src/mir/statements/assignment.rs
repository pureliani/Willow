use crate::{
    ast::expr::Expr,
    mir::{
        builders::{Builder, InBlock},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_assignment_stmt(&mut self, target: Expr, value: Expr) {
        let target_place = match self.resolve_place(target) {
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
        );

        self.write_place(&target_place, value_id);
    }
}
