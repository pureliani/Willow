use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_null_expr(
        &mut self,
        span: Span,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        if let Some(et) = expected_type {
            if let Some(variants) = self.types.get_union_variants(et.id) {
                if variants.contains(&self.types.null()) {
                    let val = self.emit_null();
                    let result = self.emit_wrap_in_union(val, &variants);
                    return self.check_expected(result, span, expected_type);
                }
            }
        }
        let result = self.emit_null();
        self.check_expected(result, span, expected_type)
    }
}
