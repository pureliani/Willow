use crate::{
    ast::expr::Expr,
    hir::{
        builders::{Builder, InBlock},
        instructions::InstrId,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_and_expr(&mut self, left: Expr, right: Expr) -> InstrId {
        let span = left.span.clone();

        let left_span = left.span.clone();
        let left_id = self.build_expr(left);

        let result = self
            .emit_logical_and(left_id, left_span, |builder| builder.build_expr(right));

        result
    }
}
