use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::InstrId,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_template_expr(&mut self, parts: Vec<Expr>, span: Span) -> InstrId {
        todo!()
    }
}
