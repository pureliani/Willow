use crate::{
    codegen::CodeGenerator,
    hir::{instructions::CastInstr, utils::adjustment::Adjustment},
};

impl<'ctx> CodeGenerator<'ctx> {
    pub fn emit_cast_instr(&mut self, instr: &CastInstr) {
        let CastInstr { src, dest, op } = instr;

        if op == &Adjustment::Identity {
            if let Some(val) = self.get_val(*src) {
                self.fn_values.insert(*dest, val);
            }
            return;
        }

        let src_val = self.get_val_strict(*src);
        let dest_ty = self.program.value_types.get(dest).unwrap();
        let dest_llvm_ty = self.lower_type(dest_ty).unwrap();

        let result = match op {
            Adjustment::Identity => unreachable!(),

            Adjustment::SExt => self
                .builder
                .build_int_cast_sign_flag(
                    src_val.into_int_value(),
                    dest_llvm_ty.into_int_type(),
                    true,
                    "sext",
                )
                .unwrap()
                .into(),

            Adjustment::ZExt => self
                .builder
                .build_int_cast_sign_flag(
                    src_val.into_int_value(),
                    dest_llvm_ty.into_int_type(),
                    false,
                    "zext",
                )
                .unwrap()
                .into(),

            Adjustment::Trunc => self
                .builder
                .build_int_truncate(
                    src_val.into_int_value(),
                    dest_llvm_ty.into_int_type(),
                    "trunc",
                )
                .unwrap()
                .into(),

            Adjustment::FExt => self
                .builder
                .build_float_ext(
                    src_val.into_float_value(),
                    dest_llvm_ty.into_float_type(),
                    "fext",
                )
                .unwrap()
                .into(),

            Adjustment::FTrunc => self
                .builder
                .build_float_trunc(
                    src_val.into_float_value(),
                    dest_llvm_ty.into_float_type(),
                    "ftrunc",
                )
                .unwrap()
                .into(),

            Adjustment::SIToF => self
                .builder
                .build_signed_int_to_float(
                    src_val.into_int_value(),
                    dest_llvm_ty.into_float_type(),
                    "sitof",
                )
                .unwrap()
                .into(),

            Adjustment::UIToF => self
                .builder
                .build_unsigned_int_to_float(
                    src_val.into_int_value(),
                    dest_llvm_ty.into_float_type(),
                    "uitof",
                )
                .unwrap()
                .into(),

            Adjustment::FToSI => self
                .builder
                .build_float_to_signed_int(
                    src_val.into_float_value(),
                    dest_llvm_ty.into_int_type(),
                    "ftosi",
                )
                .unwrap()
                .into(),

            Adjustment::FToUI => self
                .builder
                .build_float_to_unsigned_int(
                    src_val.into_float_value(),
                    dest_llvm_ty.into_int_type(),
                    "ftoui",
                )
                .unwrap()
                .into(),

            Adjustment::WrapInUnion(tag_index) => {
                let union_llvm_ty = dest_llvm_ty.into_struct_type();
                self.pack_union_variant(src_val, *tag_index as u64, union_llvm_ty)
                    .into()
            }

            Adjustment::UnwrapUnion => {
                let union_llvm_ty = self
                    .lower_type(self.program.value_types.get(src).unwrap())
                    .unwrap()
                    .into_struct_type();
                self.unpack_union_variant(
                    src_val.into_struct_value(),
                    dest_llvm_ty,
                    union_llvm_ty,
                )
            }

            Adjustment::ReTagUnion(mapping) => {
                let src_struct = src_val.into_struct_value();
                let dest_struct_ty = dest_llvm_ty.into_struct_type();
                self.retag_union(src_struct, dest_struct_ty, mapping).into()
            }

            Adjustment::CoerceStruct { field_adjustments } => {
                // TODO: implement per-field recursive casting
                todo!("CoerceStruct not yet implemented")
            }
        };

        self.fn_values.insert(*dest, result);
    }
}
