use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::IntPredicate;

use crate::{
    codegen::CodeGenerator,
    hir::{instructions::UnionInstr, types::checked_type::Type},
};

impl<'ctx> CodeGenerator<'ctx> {
    pub fn emit_union(&mut self, instr: &UnionInstr) {
        match instr {
            UnionInstr::WrapInUnion {
                dest,
                src,
                target_variants,
            } => {
                let src_val = self.get_val_strict(*src);
                let src_ty = self.program.value_types.get(src).unwrap();

                let tag_index = target_variants.iter().position(|t| t == src_ty).expect(
                    "INTERNAL COMPILER ERROR: Source type not found in Union variants",
                );

                let payload_i64 = self.cast_to_u64_payload(src_val);

                let union_ty = self.context.struct_type(
                    &[
                        self.context.i16_type().into(),
                        self.context.i64_type().into(),
                    ],
                    false,
                );

                let mut union_val = union_ty.get_undef();

                let tag_val = self.context.i16_type().const_int(tag_index as u64, false);
                union_val = self
                    .builder
                    .build_insert_value(union_val, tag_val, 0, "with_tag")
                    .unwrap()
                    .into_struct_value();

                union_val = self
                    .builder
                    .build_insert_value(union_val, payload_i64, 1, "with_payload")
                    .unwrap()
                    .into_struct_value();

                self.fn_values.insert(*dest, union_val.into());
            }

            UnionInstr::UnwrapUnion {
                dest,
                src,
                variant_type,
            } => {
                let union_val = self.get_val_strict(*src).into_struct_value();

                let payload_i64 = self
                    .builder
                    .build_extract_value(union_val, 1, "union_payload")
                    .unwrap()
                    .into_int_value();

                let dest_val = self.cast_from_u64_payload(payload_i64, variant_type);

                self.fn_values.insert(*dest, dest_val);
            }

            UnionInstr::TestVariant {
                dest,
                src,
                variant_type,
            } => {
                let union_val = self.get_val_strict(*src).into_struct_value();
                let union_ty = self.program.value_types.get(src).unwrap();

                let variants = union_ty.as_union_variants().unwrap();
                let expected_tag = variants
                    .iter()
                    .position(|t| t == variant_type)
                    .expect("Variant not found in union type");

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

            UnionInstr::WidenUnion { .. } | UnionInstr::NarrowUnion { .. } => {
                unimplemented!("Union widening/narrowing codegen not implemented yet")
            }
        }
    }

    fn cast_to_u64_payload(&self, val: BasicValueEnum<'ctx>) -> IntValue<'ctx> {
        let i64_type = self.context.i64_type();

        match val {
            BasicValueEnum::IntValue(i) => {
                let width = i.get_type().get_bit_width();
                if width == 64 {
                    i
                } else if width < 64 {
                    self.builder
                        .build_int_z_extend(i, i64_type, "zext_payload")
                        .unwrap()
                } else {
                    panic!("Cannot wrap integer larger than 64 bits in union");
                }
            }
            BasicValueEnum::FloatValue(f) => {
                let float_ty = f.get_type();
                let float_width = if float_ty == self.context.f32_type() {
                    32
                } else {
                    64
                };

                let int_type = self.context.custom_width_int_type(float_width);

                let bits = self
                    .builder
                    .build_bit_cast(f, int_type, "float_bits")
                    .unwrap()
                    .into_int_value();

                if float_width < 64 {
                    self.builder
                        .build_int_z_extend(bits, i64_type, "zext_float_payload")
                        .unwrap()
                } else {
                    bits
                }
            }
            BasicValueEnum::PointerValue(p) => self
                .builder
                .build_ptr_to_int(p, i64_type, "ptr_to_int")
                .unwrap(),
            _ => panic!("Unsupported type for union payload: {:?}", val),
        }
    }

    fn cast_from_u64_payload(
        &self,
        payload: IntValue<'ctx>,
        target_ty: &Type,
    ) -> BasicValueEnum<'ctx> {
        let llvm_target_ty = self.lower_type(target_ty).unwrap();

        match llvm_target_ty {
            inkwell::types::BasicTypeEnum::IntType(it) => {
                let width = it.get_bit_width();
                if width == 64 {
                    payload.into()
                } else {
                    self.builder
                        .build_int_truncate(payload, it, "trunc_payload")
                        .unwrap()
                        .into()
                }
            }
            inkwell::types::BasicTypeEnum::FloatType(ft) => {
                let width = if ft == self.context.f32_type() {
                    32
                } else {
                    64
                };
                let int_ty = self.context.custom_width_int_type(width);

                let bits = if width == 64 {
                    payload
                } else {
                    self.builder
                        .build_int_truncate(payload, int_ty, "trunc_float_bits")
                        .unwrap()
                };

                self.builder
                    .build_bit_cast(bits, ft, "bits_to_float")
                    .unwrap()
            }
            inkwell::types::BasicTypeEnum::PointerType(pt) => self
                .builder
                .build_int_to_ptr(payload, pt, "int_to_ptr")
                .unwrap()
                .into(),
            _ => panic!("Unsupported type for union unwrap: {:?}", target_ty),
        }
    }
}
