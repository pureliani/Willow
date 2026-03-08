use crate::{
    ast::StringNode,
    globals::STRING_INTERNER,
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{LiteralType, SpannedType, Type},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_string_literal(
        &mut self,
        node: StringNode,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let span = node.span.clone();
        let id = STRING_INTERNER.intern(&node.value);
        let literal = Type::Literal(LiteralType::String(id));

        if let Some(et) = expected_type {
            if et.kind == literal {
                return self.emit_string_literal(id);
            }

            if et.kind == Type::String {
                return self.emit_string(id);
            }

            if let Some(variants) = et.kind.get_union_variants() {
                if variants.contains(&literal) {
                    let val = self.emit_string_literal(id);
                    return self.emit_wrap_in_union(val, variants);
                }

                if variants.contains(&Type::String) {
                    let val = self.emit_string(id);
                    return self.emit_wrap_in_union(val, variants);
                }
            }
        }

        self.emit_string_literal(id)
    }
}
