use crate::hir::{
    builders::{Builder, InBlock, ValueId},
    instructions::{CastInstr, Instruction, UnionInstr},
    types::checked_type::Type,
    utils::adjustment::Adjustment,
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_wrap_in_union(
        &mut self,
        source: ValueId,
        target_type: &Type, 
    ) -> ValueId {
        let source_type = self.get_value_type(source).clone();

        let base_variants = target_type.get_base_variants().expect(
            "INTERNAL COMPILER ERROR: wrap_in_union - target is not a union",
        );

        let variant_index = base_variants.iter().position(|v| v == &source_type).expect(
            "INTERNAL COMPILER ERROR: wrap_in_union - source type is not a variant in the base",
        );

        let new_union_value = self.new_value_id(target_type.clone());

        self.push_instruction(Instruction::Cast(CastInstr {
            src: source,
            dest: new_union_value,
            op: Adjustment::WrapInUnion(variant_index),
        }));

        new_union_value
    }
    
    /// Runtime safety: The caller must ensure that at runtime the union holds a value
    /// of type `variant_type`
    pub fn emit_unwrap_from_union(
        &mut self,
        union_value: ValueId,
        variant_type: &Type,
    ) -> ValueId {
        let union_value_ty = self.get_value_type(union_value);

        let variants = union_value_ty.get_narrowed_variants().expect(
            "INTERNAL COMPILER ERROR: unwrap_from_union - union_value is not a union",
        );

        assert!(
            variants.iter().any(|v| v == variant_type),
            "INTERNAL COMPILER ERROR: unwrap_from_union - variant_type is not a member"
        );

        let payload_value = self.new_value_id(variant_type.clone());

        self.push_instruction(Instruction::Cast(CastInstr {
            src: union_value,
            dest: payload_value,
            op: Adjustment::UnwrapUnion,
        }));

        payload_value
    }

    pub fn emit_test_variant(
        &mut self,
        union_value: ValueId,
        variant_type: &Type,
    ) -> ValueId {
        let union_type = self.get_value_type(union_value);
        let variants = union_type
            .get_narrowed_variants()
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

    pub fn emit_widen_union(
        &mut self,
        union: ValueId,
        target_type: &Type,
    ) -> ValueId {
        let dest = self.new_value_id(target_type.clone());
        let source_type = self.get_value_type(union);

        let source_base = source_type.get_base_variants().expect("Expected union");
        let target_base = target_type.get_base_variants().expect("Expected union");


        if source_base == target_base {
            self.push_instruction(Instruction::Cast(CastInstr {
                src: union,
                dest,
                op: Adjustment::Identity,
            }));
        } else {
            let mut mapping = Vec::new();
            
            for (old_idx, sv) in source_base.iter().enumerate() {
                let new_idx = target_base.iter().position(|tv| sv == tv).expect(
                    "INTERNAL COMPILER ERROR: widen_union - variant missing in target base",
                );

                mapping.push((old_idx as u64, new_idx as u64));
            }

            self.push_instruction(Instruction::Cast(CastInstr {
                src: union,
                dest,
                op: Adjustment::ReTagUnion(mapping),
            }));
        }

        dest
    }

    /// Runtime safety: The caller must ensure that at runtime the union
    /// holds a variant that exists in `target_type`
    pub fn emit_narrow_union(
        &mut self,
        union: ValueId,
        target_type: &Type,
    ) -> ValueId {
        let source_type = self.get_value_type(union);

        let source_base = source_type.get_base_variants().expect("Expected union");
        let target_base = target_type.get_base_variants().expect("Expected union");
        
        assert_eq!(
            source_base, target_base, 
            "INTERNAL COMPILER ERROR: Cannot narrow to a union with a different physical base"
        );

        let dest = self.new_value_id(target_type.clone());

        self.push_instruction(Instruction::Cast(CastInstr {
            src: union,
            dest,
            op: Adjustment::Identity, 
        }));

        dest
    }
}
