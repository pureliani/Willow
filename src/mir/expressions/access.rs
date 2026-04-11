use crate::{
    ast::{expr::Expr, IdentifierNode},
    compile::interner::GenericSubstitutions,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_access_expr(
        &mut self,
        left: Expr,
        field: IdentifierNode,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let field_span = field.span.clone();
        let base_ptr = self.build_expr(left, None, substitutions);
        let field_ptr = match self.try_get_field_ptr(base_ptr, &field, false) {
            Ok(ptr) => ptr,
            Err(e) => return self.report_error_and_get_poison(e),
        };
        let field_val = self.emit_load(field_ptr);
        self.check_expected(field_val, field_span, expected_type)
    }
}
