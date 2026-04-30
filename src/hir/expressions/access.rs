use crate::{
    ast::{
        expr::{Expr, ExprKind},
        IdentifierNode,
    },
    hir::{
        builders::{Builder, InBlock},
        instructions::InstrId,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_access_expr(&mut self, left: Expr, field: IdentifierNode) -> InstrId {
        let field_span = field.span.clone();
    }
}
