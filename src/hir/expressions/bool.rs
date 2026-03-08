use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{LiteralType, SpannedType, Type},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_bool_expr(
        &mut self,
        span: Span,
        value: bool,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let literal = Type::Literal(LiteralType::Bool(value));

        if let Some(et) = expected_type {
            if et.kind == literal {
                return self.emit_bool_literal(value);
            }

            if et.kind == Type::Bool {
                return self.emit_bool(value);
            }

            if let Some(variants) = et.kind.get_union_variants() {
                if variants.contains(&literal) {
                    let val = self.emit_bool_literal(value);
                    return self.emit_wrap_in_union(val, variants);
                }

                if variants.contains(&Type::Bool) {
                    let val = self.emit_bool(value);
                    return self.emit_wrap_in_union(val, variants);
                }
            }
        }

        self.emit_bool_literal(value)
    }
}
