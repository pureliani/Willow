use std::collections::BTreeSet;

use inkwell::types::StructType;
use inkwell::IntPredicate;

use crate::{
    codegen::CodeGenerator,
    hir::{instructions::UnionInstr, types::checked_type::Type},
};

impl<'ctx> CodeGenerator<'ctx> {
    pub fn get_union_layout(&self, variants: &BTreeSet<Type>) -> (StructType<'ctx>, u64) {
        let max_size = variants
            .iter()
            .filter_map(|v| self.lower_type(v))
            .map(|ty| self.target_machine.get_target_data().get_abi_size(&ty))
            .max()
            .unwrap_or(0);

        let payload_ty = self.context.i8_type().array_type(max_size as u32);
        let struct_ty = self
            .context
            .struct_type(&[self.context.i16_type().into(), payload_ty.into()], false);

        (struct_ty, max_size)
    }

    pub fn emit_union(&mut self, instr: &UnionInstr) {
        match instr {
            UnionInstr::TestVariant {
                dest,
                src,
                variant_type,
            } => {
                let union_val = self
                    .get_val(*src)
                    .expect("INTERNAL COMPILER ERROR: TestVariant on ZST")
                    .into_struct_value();
                let union_ty = self.program.value_types.get(src).unwrap();

                let base_variants = union_ty.get_base_variants().unwrap();
                let expected_tag = base_variants
                    .iter()
                    .position(|t| t == variant_type)
                    .expect("INTERNAL COMPILER ERROR: Variant not found in union type");

                let actual_tag = self
                    .builder
                    .build_extract_value(union_val, 0, "tag")
                    .unwrap()
                    .into_int_value();

                let expected_val = self
                    .context
                    .i16_type()
                    .const_int(expected_tag as u64, false);

                let res = self.builder.build_int_compare(
                    IntPredicate::EQ,
                    actual_tag,
                    expected_val,
                    "is_variant",
                );

                self.fn_values.insert(*dest, res.unwrap().into());
            }
        }
    }
}
