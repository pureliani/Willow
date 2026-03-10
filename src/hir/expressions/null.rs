use crate::{
    ast::Span,
    hir::{
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
            if let Some(variants) = et.kind.get_narrowed_variants() {
                if variants.contains(&Type::Null) {
                    let val = self.emit_const_null();
                    return self.emit_wrap_in_union(val, &et.kind);
                }
            }
        }
        self.emit_const_null()
    }
}
