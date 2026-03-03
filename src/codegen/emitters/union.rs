use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::IntPredicate;

use crate::{
    codegen::CodeGenerator,
    globals::STRING_INTERNER,
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

                let variants = union_ty.get_union_variants().unwrap();
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

            UnionInstr::WidenUnion { dest, src }
            | UnionInstr::NarrowUnion { dest, src } => {
                let src_val = self.get_val_strict(*src).into_struct_value();
                let src_ty = self.program.value_types.get(src).unwrap();
                let dest_ty = self.program.value_types.get(dest).unwrap();

                let src_variants = src_ty
                    .get_union_variants()
                    .expect("INTERNAL COMPILER ERROR: Widen/Narrow src is not a union");
                let dest_variants = dest_ty
                    .get_union_variants()
                    .expect("INTERNAL COMPILER ERROR: Widen/Narrow dest is not a union");

                let old_tag = self
                    .builder
                    .build_extract_value(src_val, 0, "old_tag")
                    .unwrap()
                    .into_int_value();
                let payload = self
                    .builder
                    .build_extract_value(src_val, 1, "payload")
                    .unwrap();

                let mut mapping = Vec::new();
                for (old_idx, variant) in src_variants.iter().enumerate() {
                    if let Some(new_idx) = dest_variants.iter().position(|v| v == variant)
                    {
                        mapping.push((old_idx as u64, new_idx as u64));
                    }
                }

                let is_identity = mapping.iter().all(|(o, n)| o == n);

                let new_tag = if is_identity {
                    old_tag
                } else {
                    self.emit_tag_switch(old_tag, &mapping)
                };

                let union_ty = self.context.struct_type(
                    &[
                        self.context.i16_type().into(),
                        self.context.i64_type().into(),
                    ],
                    false,
                );

                let mut res = union_ty.get_undef();
                res = self
                    .builder
                    .build_insert_value(res, new_tag, 0, "new_tag")
                    .unwrap()
                    .into_struct_value();
                res = self
                    .builder
                    .build_insert_value(res, payload, 1, "new_payload")
                    .unwrap()
                    .into_struct_value();

                self.fn_values.insert(*dest, res.into());
            }
        }
    }

    fn emit_tag_switch(
        &mut self,
        old_tag: IntValue<'ctx>,
        mapping: &[(u64, u64)],
    ) -> IntValue<'ctx> {
        let fn_name = STRING_INTERNER.resolve(self.current_fn.unwrap().identifier.name);
        let current_fn = self.module.get_function(&fn_name).unwrap();

        let switch_bb = self.builder.get_insert_block().unwrap();
        let merge_bb = self.context.append_basic_block(current_fn, "tag_map_merge");
        let default_bb = self
            .context
            .append_basic_block(current_fn, "tag_map_default");

        self.builder.position_at_end(default_bb);
        self.builder.build_unreachable().unwrap();

        let mut cases = Vec::with_capacity(mapping.len());
        let mut incoming_phis = Vec::with_capacity(mapping.len());

        for (old_idx, new_idx) in mapping {
            let case_bb = self.context.append_basic_block(
                current_fn,
                &format!("map_{}_to_{}", old_idx, new_idx),
            );
            self.builder.position_at_end(case_bb);

            let new_tag_val = self.context.i16_type().const_int(*new_idx, false);
            self.builder.build_unconditional_branch(merge_bb).unwrap();

            cases.push((self.context.i16_type().const_int(*old_idx, false), case_bb));
            incoming_phis.push((new_tag_val, case_bb));
        }

        self.builder.position_at_end(switch_bb);
        self.builder
            .build_switch(old_tag, default_bb, &cases)
            .unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.context.i16_type(), "remapped_tag")
            .unwrap();

        for (val, bb) in incoming_phis {
            phi.add_incoming(&[(&val, bb)]);
        }

        phi.as_basic_value().into_int_value()
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
