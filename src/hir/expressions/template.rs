use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{SpannedType, StructKind, Type},
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_template_expr(
        &mut self,
        parts: Vec<Expr>,
        span: Span,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        todo!()
    }
}
