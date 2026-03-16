use crate::{
    ast::Span,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_bool_expr(
        &mut self,
        span: Span,
        value: bool,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        self.emit_bool(value)
    }
}
