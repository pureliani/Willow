use crate::{
    ast::Span,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{SpannedType, Type},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_null_expr(
        &mut self,
        span: Span,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        if let Some(et) = expected_type {
            if let Some(variants) = et.kind.get_union_variants() {
                if variants.contains(&Type::Null) {
                    let val = self.emit_null();
                    let result = self.wrap_in_union(val, variants);
                    return self.check_expected(result, span, expected_type);
                }
            }
        }
        let result = self.emit_null();
        self.check_expected(result, span, expected_type)
    }
}
