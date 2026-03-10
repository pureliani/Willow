use inkwell::{
    types::{BasicTypeEnum, StructType},
    values::{BasicValue, BasicValueEnum, IntValue, PointerValue},
    IntPredicate,
};

use crate::{
    codegen::CodeGenerator,
    hir::{builders::ValueId, instructions::ListInstr, types::checked_type::Type},
};

const DATA_PTR_FIELD_INDEX: u32 = 0;
const LEN_FIELD_INDEX: u32 = 1;
const CAP_FIELD_INDEX: u32 = 2;

impl<'ctx> CodeGenerator<'ctx> {
    pub fn get_list_header_layout(&mut self) -> StructType<'ctx> {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();

        self.context.struct_type(
            &[
                ptr_type.into(), // Data Pointer
                i64_type.into(), // Len
                i64_type.into(), // Cap
            ],
            false,
        )
    }

    fn load_list_len(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        header_type: StructType<'ctx>,
    ) -> IntValue<'ctx> {
        let len_field = self
            .builder
            .build_struct_gep(header_type, list_ptr, LEN_FIELD_INDEX, "len_field")
            .unwrap();

        self.builder
            .build_load(self.context.i64_type(), len_field, "len")
            .unwrap()
            .into_int_value()
    }

    fn load_list_data_ptr(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        header_type: StructType<'ctx>,
    ) -> PointerValue<'ctx> {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        let data_field = self
            .builder
            .build_struct_gep(header_type, list_ptr, DATA_PTR_FIELD_INDEX, "data_field")
            .unwrap();

        self.builder
            .build_load(ptr_type, data_field, "data_ptr")
            .unwrap()
            .into_pointer_value()
    }

    fn load_element_at(
        &mut self,
        data_ptr: PointerValue<'ctx>,
        index: IntValue<'ctx>,
        elem_llvm_ty: BasicTypeEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let elem_ptr = unsafe {
            self.builder
                .build_gep(elem_llvm_ty, data_ptr, &[index], "elem_ptr")
                .unwrap()
        };

        self.builder
            .build_load(elem_llvm_ty, elem_ptr, "elem")
            .unwrap()
    }

    fn store_element_at(
        &mut self,
        data_ptr: PointerValue<'ctx>,
        index: IntValue<'ctx>,
        elem_llvm_ty: BasicTypeEnum<'ctx>,
        value: BasicValueEnum<'ctx>,
    ) {
        let elem_ptr = unsafe {
            self.builder
                .build_gep(elem_llvm_ty, data_ptr, &[index], "elem_ptr")
                .unwrap()
        };

        self.builder.build_store(elem_ptr, value).unwrap();
    }
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn emit_list(&mut self, instr: &ListInstr) {
        match instr {
            ListInstr::Init {
                dest,
                element_type,
                items,
            } => self.emit_list_init(*dest, element_type, items),

            ListInstr::Get { dest, list, index } => {
                self.emit_list_get(*dest, *list, *index);
            }

            ListInstr::GetUnsafe { dest, list, index } => {
                self.emit_list_get_unsafe(*dest, *list, *index);
            }

            ListInstr::Set {
                dest,
                list,
                index,
                value,
            } => self.emit_list_set(*dest, *list, *index, *value),

            ListInstr::Len { dest, list } => {
                self.emit_list_len(*dest, *list);
            }
        }
    }
}

impl<'ctx> CodeGenerator<'ctx> {
    fn emit_list_init(&mut self, dest: ValueId, element_type: &Type, items: &[ValueId]) {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let header_type = self.get_list_header_layout();

        let header_ptr = self
            .builder
            .build_malloc(header_type, "list_header")
            .unwrap();

        let count = items.len() as u64;
        let count_val = i64_type.const_int(count, false);

        let data_ptr = match self.lower_type(element_type) {
            Some(elem_llvm_ty) => {
                let ptr = self
                    .builder
                    .build_array_malloc(elem_llvm_ty, count_val, "list_data")
                    .unwrap();

                for (i, item_id) in items.iter().enumerate() {
                    let val = self.get_val(*item_id).expect(
                        "INTERNAL COMPILER ERROR: Non-ZST list element has no runtime \
                         value",
                    );
                    let offset = i64_type.const_int(i as u64, false);
                    self.store_element_at(ptr, offset, elem_llvm_ty, val);
                }

                ptr
            }
            None => ptr_type.const_null(),
        };

        let data_field = self
            .builder
            .build_struct_gep(header_type, header_ptr, DATA_PTR_FIELD_INDEX, "ptr_field")
            .unwrap();
        self.builder.build_store(data_field, data_ptr).unwrap();

        let len_field = self
            .builder
            .build_struct_gep(header_type, header_ptr, LEN_FIELD_INDEX, "len_field")
            .unwrap();
        self.builder.build_store(len_field, count_val).unwrap();

        let cap_field = self
            .builder
            .build_struct_gep(header_type, header_ptr, CAP_FIELD_INDEX, "cap_field")
            .unwrap();
        self.builder.build_store(cap_field, count_val).unwrap();

        self.fn_values
            .insert(dest, header_ptr.as_basic_value_enum());
    }
}

impl<'ctx> CodeGenerator<'ctx> {
    fn emit_list_get(&mut self, dest: ValueId, list: ValueId, index: ValueId) {
        let list_ptr = self.get_val_strict(list).into_pointer_value();
        let idx_val = self.get_val_strict(index).into_int_value();
        let header_type = self.get_list_header_layout();

        let len = self.load_list_len(list_ptr, header_type);
        let in_bounds = self
            .builder
            .build_int_compare(IntPredicate::ULT, idx_val, len, "in_bounds")
            .unwrap();

        let dest_ty = self.program.value_types.get(&dest).unwrap().clone();
        let variants = dest_ty
            .get_narrowed_variants()
            .expect("INTERNAL COMPILER ERROR: List Get dest must be a union");

        let list_ty = self.program.value_types.get(&list).unwrap();
        let Type::List(inner) = list_ty else {
            unreachable!()
        };
        let elem_ty = &inner.kind;

        let null_tag = self.find_variant_tag(variants, &Type::Null);
        let (union_llvm_ty, _) = self.get_union_layout(variants);

        if self.lower_type(elem_ty).is_none() {
            let elem_tag = self.find_variant_tag(variants, elem_ty);
            let selected_tag = self
                .builder
                .build_select(
                    in_bounds,
                    self.context.i16_type().const_int(elem_tag, false),
                    self.context.i16_type().const_int(null_tag, false),
                    "tag_select",
                )
                .unwrap();

            let result =
                self.build_zst_union(selected_tag.into_int_value(), union_llvm_ty);
            self.fn_values.insert(dest, result.into());
            return;
        }

        // Non-ZST, branch on bounds
        let data_ptr = self.load_list_data_ptr(list_ptr, header_type);
        let elem_llvm_ty = self.lower_type(elem_ty).unwrap();
        let null_tag_val = self.context.i16_type().const_int(null_tag, false);

        let result = self.build_conditional_value(
            in_bounds,
            union_llvm_ty.into(),
            |cg| {
                let elem_val = cg.load_element_at(data_ptr, idx_val, elem_llvm_ty);

                if let Some(elem_variants) = elem_ty.get_narrowed_variants() {
                    let mut mapping: Vec<(u64, u64)> = Vec::new();
                    for (old_idx, sv) in elem_variants.iter().enumerate() {
                        let new_idx = cg.find_variant_tag(variants, sv);
                        mapping.push((old_idx.try_into().unwrap(), new_idx));
                    }
                    cg.retag_union(elem_val.into_struct_value(), union_llvm_ty, &mapping)
                        .into()
                } else {
                    let elem_tag = cg.find_variant_tag(variants, elem_ty);
                    cg.pack_union_variant(elem_val, elem_tag, union_llvm_ty)
                        .into()
                }
            },
            |cg| cg.build_zst_union(null_tag_val, union_llvm_ty).into(),
            "get",
        );

        self.fn_values.insert(dest, result);
    }
}

impl<'ctx> CodeGenerator<'ctx> {
    fn emit_list_get_unsafe(&mut self, dest: ValueId, list: ValueId, index: ValueId) {
        let dest_ty = self.program.value_types.get(&dest).unwrap();

        if self.lower_type(dest_ty).is_none() {
            return;
        }

        let list_ptr = self.get_val_strict(list).into_pointer_value();
        let idx_val = self.get_val_strict(index).into_int_value();
        let header_type = self.get_list_header_layout();

        let data_ptr = self.load_list_data_ptr(list_ptr, header_type);
        let elem_llvm_ty = self.lower_type(dest_ty).unwrap();
        let val = self.load_element_at(data_ptr, idx_val, elem_llvm_ty);

        self.fn_values.insert(dest, val);
    }
}

impl<'ctx> CodeGenerator<'ctx> {
    fn emit_list_set(
        &mut self,
        dest: ValueId,
        list: ValueId,
        index: ValueId,
        value: ValueId,
    ) {
        let val_ty = self.program.value_types.get(&value).unwrap();
        let list_val = self.get_val_strict(list);

        if self.lower_type(val_ty).is_none() {
            // ZST - store is a no-op, dest aliases the same list
            self.fn_values.insert(dest, list_val);
            return;
        }

        let list_ptr = list_val.into_pointer_value();
        let idx_val = self.get_val_strict(index).into_int_value();
        let new_val = self.get_val_strict(value);

        let header_type = self.get_list_header_layout();
        let data_ptr = self.load_list_data_ptr(list_ptr, header_type);
        let elem_llvm_ty = self.lower_type(val_ty).unwrap();

        self.store_element_at(data_ptr, idx_val, elem_llvm_ty, new_val);

        self.fn_values.insert(dest, list_ptr.as_basic_value_enum());
    }
}

impl<'ctx> CodeGenerator<'ctx> {
    fn emit_list_len(&mut self, dest: ValueId, list: ValueId) {
        let list_ptr = self.get_val_strict(list).into_pointer_value();
        let header_type = self.get_list_header_layout();
        let len = self.load_list_len(list_ptr, header_type);
        self.fn_values.insert(dest, len.into());
    }
}
