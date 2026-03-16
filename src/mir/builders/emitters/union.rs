use std::collections::BTreeSet;

use crate::{
    globals::COMMON_IDENTIFIERS,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{StructKind, Type},
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    /// Wraps a single variant value into a union. Stack-allocates the union
    /// struct, writes the discriminant and payload.
    pub fn wrap_in_union(
        &mut self,
        source: ValueId,
        variants: &BTreeSet<Type>,
    ) -> ValueId {
        let source_type = self.get_value_type(source).clone();

        let variant_index = variants
            .iter()
            .position(|v| v == &source_type)
            .unwrap_or_else(|| {
                panic!(
                    "INTERNAL COMPILER ERROR: wrap_in_union - source type is not a \
                     variant of the union"
                )
            });

        let struct_type = Type::Struct(StructKind::TaggedUnion(variants.clone()));
        let union_ptr = self.emit_stack_alloc(struct_type, 1);

        let id_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.id);
        let id_val = self.emit_number(NumberKind::U16(variant_index as u16));
        self.emit_store(id_ptr, id_val);

        let value_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.val);
        let typed_ptr =
            self.emit_bitcast_unsafe(value_ptr, Type::Pointer(Box::new(source_type)));
        self.emit_store(typed_ptr, source);

        union_ptr
    }

    /// Extracts a variant value from a union pointer. Caller must ensure
    /// the active variant matches `variant_type` (via test_variant or
    /// known assignment).
    pub fn unwrap_from_union(
        &mut self,
        union_ptr: ValueId,
        variant_type: &Type,
    ) -> ValueId {
        let union_ptr_ty = self.get_value_type(union_ptr);
        assert!(
            union_ptr_ty
                .get_union_variants()
                .expect(
                    "INTERNAL COMPILER ERROR: unwrap_from_union - union_ptr is not \
                     pointing to a union"
                )
                .iter()
                .any(|v| { v == variant_type }),
            "INTERNAL COMPILER ERROR: unwrap_from_union - variant_type is not a \
             member of the union"
        );

        let value_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.val);
        let typed_ptr = self.emit_bitcast_unsafe(
            value_ptr,
            Type::Pointer(Box::new(variant_type.clone())),
        );
        self.emit_load(typed_ptr)
    }

    /// Tests whether a union value holds a specific variant. Returns a
    /// bool ValueId.
    pub fn test_variant(&mut self, union_ptr: ValueId, variant_type: &Type) -> ValueId {
        let union_type = self.get_value_type(union_ptr).clone();
        let variants = union_type
            .get_union_variants()
            .expect("INTERNAL COMPILER ERROR: test_variant called with non-union");

        let variant_index = variants
            .iter()
            .position(|v| v == variant_type)
            .expect("INTERNAL COMPILER ERROR: variant not found in union");

        let id_ptr = self.get_field_ptr(union_ptr, COMMON_IDENTIFIERS.id);
        let id_val = self.emit_load(id_ptr);
        let expected = self.emit_number(NumberKind::U16(variant_index as u16));
        self.eq(id_val, expected)
    }

    /// Widens a union to a larger union that contains all source variants
    /// plus additional ones. Copies the payload and remaps the discriminant.
    pub fn widen_union(
        &mut self,
        source_ptr: ValueId,
        source_variants: &BTreeSet<Type>,
        target_variants: &BTreeSet<Type>,
    ) -> ValueId {
        assert!(
            source_variants.len() <= target_variants.len(),
            "INTERNAL COMPILER ERROR: widen_union called but source has more \
             variants than target"
        );

        for sv in source_variants {
            assert!(
                target_variants.iter().any(|tv| sv == tv),
                "INTERNAL COMPILER ERROR: widen_union - source variant not found \
                 in target union"
            );
        }

        let remap = self.build_variant_remap(source_variants, target_variants);

        let target_struct =
            Type::Struct(StructKind::TaggedUnion(target_variants.clone()));
        let target_ptr = self.emit_stack_alloc(target_struct, 1);

        let src_value_ptr = self.get_field_ptr(source_ptr, COMMON_IDENTIFIERS.val);
        let dst_value_ptr = self.get_field_ptr(target_ptr, COMMON_IDENTIFIERS.val);
        self.emit_memcopy(src_value_ptr, dst_value_ptr);

        self.emit_discriminant_remap(source_ptr, target_ptr, &remap);

        target_ptr
    }

    /// Narrows a union to a smaller union that contains a subset of the
    /// source variants. Copies the payload and remaps the discriminant.
    ///
    /// The caller must ensure that the runtime active variant is one of
    /// the target variants (via a prior type check). This method validates
    /// the structural precondition (target ⊂ source) but cannot validate
    /// the runtime invariant.
    pub fn narrow_union(
        &mut self,
        source_ptr: ValueId,
        source_variants: &BTreeSet<Type>,
        target_variants: &BTreeSet<Type>,
    ) -> ValueId {
        assert!(
            target_variants.len() < source_variants.len(),
            "INTERNAL COMPILER ERROR: narrow_union called but target has >= \
             variants than source (use widen_union instead)"
        );

        for tv in target_variants {
            assert!(
                source_variants.iter().any(|sv| sv == tv),
                "INTERNAL COMPILER ERROR: narrow_union - target variant not found \
                 in source union"
            );
        }

        // Remap: for each source variant that exists in target, map
        // source_index -> target_index. Source variants not in target
        // are skipped (they can't be active at runtime).
        let remap: Vec<(u16, u16)> = source_variants
            .iter()
            .enumerate()
            .filter_map(|(src_idx, variant)| {
                let tgt_idx = target_variants.iter().position(|v| v == variant);
                tgt_idx.map(|ti| (src_idx as u16, ti as u16))
            })
            .collect();

        let target_struct =
            Type::Struct(StructKind::TaggedUnion(target_variants.clone()));
        let target_ptr = self.emit_stack_alloc(target_struct, 1);

        // Source payload >= target payload in size. The active variant
        // fits in the target buffer. Bitcast the source payload to the
        // target payload type so memcopy uses the target (smaller) size.
        let src_value_ptr = self.get_field_ptr(source_ptr, COMMON_IDENTIFIERS.val);
        let dst_value_ptr = self.get_field_ptr(target_ptr, COMMON_IDENTIFIERS.val);

        let src_as_target_payload = self.emit_bitcast_unsafe(
            src_value_ptr,
            Type::Pointer(Box::new(Type::TaglessUnion(target_variants.clone()))),
        );
        self.emit_memcopy(src_as_target_payload, dst_value_ptr);

        self.emit_discriminant_remap(source_ptr, target_ptr, &remap);

        target_ptr
    }

    /// Builds the index remap table: for each source variant, find its
    /// position in the target variant set.
    fn build_variant_remap(
        &self,
        source_variants: &BTreeSet<Type>,
        target_variants: &BTreeSet<Type>,
    ) -> Vec<(u16, u16)> {
        source_variants
            .iter()
            .enumerate()
            .map(|(src_idx, variant)| {
                let tgt_idx = target_variants.iter().position(|v| v == variant).expect(
                    "INTERNAL COMPILER ERROR: source variant not found in target \
                         union",
                );
                (src_idx as u16, tgt_idx as u16)
            })
            .collect()
    }

    /// Emits the select chain that remaps the discriminant from source
    /// indices to target indices, then stores it in the target union.
    fn emit_discriminant_remap(
        &mut self,
        source_ptr: ValueId,
        target_ptr: ValueId,
        remap: &[(u16, u16)],
    ) {
        let src_id_ptr = self.get_field_ptr(source_ptr, COMMON_IDENTIFIERS.id);
        let src_id = self.emit_load(src_id_ptr);

        let mut remapped_id = self.emit_number(NumberKind::U16(remap.last().unwrap().1));

        for &(src_idx, tgt_idx) in remap.iter().rev().skip(1) {
            let cmp_val = self.emit_number(NumberKind::U16(src_idx));
            let is_match = self.eq(src_id, cmp_val);
            let tgt_val = self.emit_number(NumberKind::U16(tgt_idx));
            remapped_id = self.emit_select(is_match, tgt_val, remapped_id);
        }

        let dst_id_ptr = self.get_field_ptr(target_ptr, COMMON_IDENTIFIERS.id);
        self.emit_store(dst_id_ptr, remapped_id);
    }
}
