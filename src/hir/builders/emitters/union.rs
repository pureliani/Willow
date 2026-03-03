use crate::hir::{
    builders::{Builder, InBlock, ValueId},
    instructions::{Instruction, UnionInstr},
    types::checked_type::Type,
};

impl<'a> Builder<'a, InBlock> {
    /// Tests whether a union value holds a specific variant. Returns a
    /// bool ValueId.
    pub fn emit_test_variant(
        &mut self,
        union_value: ValueId,
        variant_type: &Type,
    ) -> ValueId {
        let union_type = self.get_value_type(union_value);
        let variants = union_type
            .get_union_variants()
            .expect("INTERNAL COMPILER ERROR: test_variant called with non-union");

        assert!(
            variants.iter().any(|v| v == variant_type),
            "INTERNAL COMPILER ERROR: variant not found in union"
        );

        let bool_value = self.new_value_id(Type::Bool);
        self.push_instruction(Instruction::Union(UnionInstr::TestVariant {
            dest: bool_value,
            src: union_value,
            variant_type: variant_type.clone(),
        }));
        bool_value
    }
}
