use inkwell::types::BasicTypeEnum;
use inkwell::AddressSpace;

use crate::codegen::CodeGenerator;
use crate::compile::interner::TypeId;
use crate::mir::types::checked_type::{StructKind, Type};
use crate::mir::utils::layout::get_layout_of;

impl<'ctx, 'a> CodeGenerator<'ctx, 'a> {
    pub fn get_basic_type(&self, ty_id: TypeId) -> BasicTypeEnum<'ctx> {
        let ty = self.type_interner.resolve(ty_id);

        let layout = get_layout_of(
            &ty,
            self.type_interner,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if layout.size == 0 {
            return self.context.struct_type(&[], false).into();
        }

        match ty {
            Type::Bool => self.context.bool_type().into(),
            Type::U8 | Type::I8 => self.context.i8_type().into(),
            Type::U16 | Type::I16 => self.context.i16_type().into(),
            Type::U32 | Type::I32 => self.context.i32_type().into(),
            Type::U64 | Type::I64 => self.context.i64_type().into(),
            Type::USize | Type::ISize => {
                let target_data = self.target_machine.get_target_data();
                self.context.ptr_sized_int_type(&target_data, None).into()
            }
            Type::F32 => self.context.f32_type().into(),
            Type::F64 => self.context.f64_type().into(),
            Type::Pointer(_) | Type::IndirectFn(_) => {
                self.context.ptr_type(AddressSpace::default()).into()
            }
            Type::TaglessUnion(_) => {
                let i8_ty = self.context.i8_type();
                i8_ty.array_type(layout.size as u32).into()
            }
            Type::Struct(s) => {
                if let StructKind::String = s {
                    let target_data = self.target_machine.get_target_data();
                    let usize_ty = self.context.ptr_sized_int_type(&target_data, None);
                    let flexible_array_ty = self.context.i8_type().array_type(0);

                    return self
                        .context
                        .struct_type(&[usize_ty.into(), flexible_array_ty.into()], false)
                        .into();
                }

                let fields = s.fields(self.type_interner);
                let mut field_types = Vec::new();
                for (_, f_id) in fields {
                    let f_ty = self.get_basic_type(f_id);
                    field_types.push(f_ty);
                }
                self.context.struct_type(&field_types, false).into()
            }
            _ => unreachable!("ZSTs handled above, physical types handled here"),
        }
    }
}
