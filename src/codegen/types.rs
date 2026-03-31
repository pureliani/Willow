use inkwell::types::{AnyType, AnyTypeEnum, BasicTypeEnum};
use inkwell::AddressSpace;

use crate::codegen::CodeGenerator;
use crate::compile::interner::TypeId;
use crate::mir::types::checked_declaration::FnType;
use crate::mir::types::checked_type::{StructKind, Type};
use crate::mir::utils::layout::get_layout_of;

impl<'ctx, 'a> CodeGenerator<'ctx, 'a> {
    /// Converts a Willow TypeId into an LLVM AnyTypeEnum
    /// this is specifically for function return types
    pub fn get_any_type(&self, ty_id: TypeId) -> AnyTypeEnum<'ctx> {
        let ty = self.type_interner.resolve(ty_id);
        match ty {
            Type::Void | Type::Never => self.context.void_type().into(),
            _ => self.get_basic_type(ty_id).as_any_type_enum(),
        }
    }

    /// Converts a Willow TypeId into an LLVM BasicTypeEnum
    /// These are types that can be stored in variables, struct fields, or passed as arguments
    pub fn get_basic_type(&self, ty_id: TypeId) -> BasicTypeEnum<'ctx> {
        let ty = self.type_interner.resolve(ty_id);
        match ty {
            // We represent Willow's Void, Null, and Never as ZSTs
            Type::Void | Type::Null | Type::Never => {
                self.context.struct_type(&[], false).into()
            }
            Type::Bool(_) => self.context.bool_type().into(),
            Type::U8(_) | Type::I8(_) => self.context.i8_type().into(),
            Type::U16(_) | Type::I16(_) => self.context.i16_type().into(),
            Type::U32(_) | Type::I32(_) => self.context.i32_type().into(),
            Type::U64(_) | Type::I64(_) => self.context.i64_type().into(),
            Type::USize(_) | Type::ISize(_) => {
                let target_data = self.target_machine.get_target_data();
                self.context.ptr_sized_int_type(&target_data, None).into()
            }
            Type::F32(_) => self.context.f32_type().into(),
            Type::F64(_) => self.context.f64_type().into(),
            Type::Pointer(_) => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Fn(fntype) => match fntype {
                // Direct function references carry no state at runtime (ZST)
                FnType::Direct(_) => self.context.struct_type(&[], false).into(),
                // Indirect function references are pointers
                FnType::Indirect { .. } => {
                    self.context.ptr_type(AddressSpace::default()).into()
                }
            },
            Type::TaglessUnion(_) => {
                let layout = get_layout_of(
                    &ty,
                    self.type_interner,
                    self.program.target_ptr_size,
                    self.program.target_ptr_align,
                );
                let i8_ty = self.context.i8_type();
                i8_ty.array_type(layout.size as u32).into()
            }
            Type::Struct(s) => {
                if let StructKind::StringHeader(Some(_)) = s {
                    return self.context.struct_type(&[], false).into();
                }

                let fields = s.fields(self.type_interner);
                let mut field_types = Vec::with_capacity(fields.len());

                for (_, field_ty_id) in fields {
                    field_types.push(self.get_basic_type(field_ty_id));
                }

                self.context.struct_type(&field_types, false).into()
            }
            Type::Unknown => {
                panic!("INTERNAL COMPILER ERROR: Cannot lower Unknown type to LLVM IR")
            }
        }
    }
}
