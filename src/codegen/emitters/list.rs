use inkwell::{types::StructType, values::BasicValue};

use crate::{
    codegen::CodeGenerator,
    hir::{instructions::ListInstr, types::checked_type::Type},
};

const DATA_PTR_FIELD_INDEX: u32 = 0;
const LEN_FIELD_INDEX: u32 = 1;
const CAP_FIELD_INDEX: u32 = 2;

impl<'ctx> CodeGenerator<'ctx> {
    pub fn get_list_header_layout(&mut self) -> StructType<'ctx> {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let header_type = self.context.struct_type(
            &[
                ptr_type.into(), // Data Pointer
                i64_type.into(), // Len
                i64_type.into(), // Cap
            ],
            false,
        );

        header_type
    }
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn emit_list(&mut self, instr: &ListInstr) {
        match instr {
            ListInstr::Init {
                dest,
                element_type,
                items,
            } => {
                let i64_type = self.context.i64_type();
                let header_type = self.get_list_header_layout();
                let header_ptr = self
                    .builder
                    .build_malloc(header_type, "list_header")
                    .unwrap();

                let count = items.len() as u64;
                let count_val = i64_type.const_int(count, false);

                let elem_llvm_type = self.lower_type(element_type).unwrap();
                let data_ptr = self
                    .builder
                    .build_array_malloc(elem_llvm_type, count_val, "list_data")
                    .unwrap();

                for (i, item_id) in items.iter().enumerate() {
                    let val = self.get_val_strict(*item_id);
                    let item_ty = self.program.value_types.get(item_id).unwrap();

                    if item_ty != element_type {
                        panic!("INTERNAL COMPILER ERROR: List init type mismatch. Expected {:?}, got {:?}", element_type, item_ty);
                    }

                    unsafe {
                        let item_ptr = self
                            .builder
                            .build_gep(
                                elem_llvm_type,
                                data_ptr,
                                &[i64_type.const_int(i as u64, false)],
                                "item_ptr",
                            )
                            .unwrap();
                        self.builder.build_store(item_ptr, val).unwrap();
                    }
                }

                // Data Pointer
                let data_ptr_field_ptr = self
                    .builder
                    .build_struct_gep(
                        header_type,
                        header_ptr,
                        DATA_PTR_FIELD_INDEX,
                        "ptr_field",
                    )
                    .unwrap();
                self.builder
                    .build_store(data_ptr_field_ptr, data_ptr)
                    .unwrap();

                // Len
                let len_field_ptr = self
                    .builder
                    .build_struct_gep(
                        header_type,
                        header_ptr,
                        LEN_FIELD_INDEX,
                        "len_field",
                    )
                    .unwrap();
                self.builder.build_store(len_field_ptr, count_val).unwrap();

                // Cap
                let cap_field_ptr = self
                    .builder
                    .build_struct_gep(
                        header_type,
                        header_ptr,
                        CAP_FIELD_INDEX,
                        "cap_field",
                    )
                    .unwrap();
                self.builder.build_store(cap_field_ptr, count_val).unwrap();

                self.fn_values
                    .insert(*dest, header_ptr.as_basic_value_enum());
            }
            ListInstr::GetUnsafe { dest, list, index } => todo!(),
            ListInstr::Get { dest, list, index } => todo!(),
            ListInstr::Set {
                dest,
                list,
                index,
                value,
            } => {
                todo!()
            }
            ListInstr::Len { dest, list } => {
                let list_ptr = self.get_val_strict(*list).into_pointer_value();
                let i64_type = self.context.i64_type();
                let header_type = self.get_list_header_layout();

                let len_ptr = self
                    .builder
                    .build_struct_gep(header_type, list_ptr, LEN_FIELD_INDEX, "len_ptr")
                    .unwrap();

                let len = self.builder.build_load(i64_type, len_ptr, "len").unwrap();
                self.fn_values.insert(*dest, len);
            }
        }
    }
}
