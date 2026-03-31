use std::collections::BTreeSet;

use crate::{
    compile::interner::TypeId,
    globals::COMMON_IDENTIFIERS,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{StructKind, Type},
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    /// Wraps a single variant value into a union. Stack-allocates the union
    /// struct, writes the discriminant and payload, and returns the union value
    pub fn emit_wrap_in_union(
        &mut self,
        source: ValueId,
        variants: &BTreeSet<TypeId>,
    ) -> ValueId {
        let source_type = self.get_value_type(source);

        assert!(
            self.types.get_union_variants(source_type).is_none(),
            "INTERNAL COMPILER ERROR: emit_wrap_in_union called with a union source type. \
             Use coerce_to_union instead."
        );

        assert!(
            variants.contains(&source_type),
            "INTERNAL COMPILER ERROR: emit_wrap_in_union - source type is not a \
             variant of the union"
        );

        let struct_type = self
            .types
            .intern(&Type::Struct(StructKind::TaggedUnion(variants.clone())));
        let union_ptr = self.emit_stack_alloc(struct_type, 1);

        let id_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.id);
        let id_val = self.emit_number(NumberKind::U32(source_type.0));
        let wide_u32 = self.types.u32(None);
        let id_val_wide = self.emit_bitcast(id_val, wide_u32);
        self.emit_store(id_ptr, id_val_wide);

        let value_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.val);
        let typed_ptr = self.emit_bitcast(value_ptr, self.types.ptr(source_type));
        self.emit_store(typed_ptr, source);

        self.emit_load(union_ptr)
    }

    /// Extracts a variant value from a union value. Caller must ensure
    /// the active variant matches `variant_type` (via emit_test_variant or
    /// known assignment)
    pub fn emit_unwrap_from_union(
        &mut self,
        union_val: ValueId,
        variant_type: TypeId,
    ) -> ValueId {
        let union_ty = self.get_value_type(union_val);

        assert!(
            !self.types.is_pointer(union_ty),
            "INTERNAL COMPILER ERROR: emit_unwrap_from_union expected union to be passed by value, \
             but got a pointer"
        );

        let variants = self.types.get_union_variants(union_ty).expect(
            "INTERNAL COMPILER ERROR: emit_unwrap_from_union - union_val is not \
                 a union",
        );

        assert!(
            variants.contains(&variant_type),
            "INTERNAL COMPILER ERROR: emit_unwrap_from_union - variant_type is not a \
             member of the union"
        );

        assert!(
            self.types.get_union_variants(variant_type).is_none(),
            "INTERNAL COMPILER ERROR: emit_unwrap_from_union - cannot unwrap to a union type. \
             Use coerce_to_union instead."
        );

        let union_ptr = self.emit_stack_alloc(union_ty, 1);
        self.emit_store(union_ptr, union_val);

        let value_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.val);
        let typed_ptr = self.emit_bitcast(value_ptr, self.types.ptr(variant_type));
        self.emit_load(typed_ptr)
    }

    /// Tests whether a union value holds a specific variant. Returns a
    /// bool ValueId
    pub fn emit_test_variant(
        &mut self,
        union_val: ValueId,
        variant_type: TypeId,
    ) -> ValueId {
        let union_ty = self.get_value_type(union_val);

        assert!(
            !self.types.is_pointer(union_ty),
            "INTERNAL COMPILER ERROR: emit_test_variant expected union to be passed by value, \
             but got a pointer"
        );

        let variants = self
            .types
            .get_union_variants(union_ty)
            .expect("INTERNAL COMPILER ERROR: emit_test_variant called with non-union");

        assert!(
            variants.contains(&variant_type),
            "INTERNAL COMPILER ERROR: variant not found in union"
        );

        assert!(
            self.types.get_union_variants(variant_type).is_none(),
            "INTERNAL COMPILER ERROR: emit_test_variant - cannot test for a union type."
        );

        let union_ptr = self.emit_stack_alloc(union_ty, 1);
        self.emit_store(union_ptr, union_val);

        let id_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.id);
        let id_val = self.emit_load(id_ptr);
        let expected = self.emit_number(NumberKind::U32(variant_type.0));
        let wide_u32 = self.types.u32(None);
        let expected_wide = self.emit_bitcast(expected, wide_u32);

        self.eq(id_val, expected_wide)
    }
}
