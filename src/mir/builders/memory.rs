use crate::{
    ast::{IdentifierNode, Span},
    compile::interner::{StringId, TypeId},
    globals::{COMMON_IDENTIFIERS, STRING_INTERNER},
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{Instruction, MemoryInstr},
        types::checked_type::{StructKind, Type},
        utils::layout::get_layout_of,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_stack_alloc(&mut self, value_type: TypeId, count: usize) -> ValueId {
        let ptr_ty = self.types.ptr(value_type);
        let dest = self.new_value_id(ptr_ty);

        let ptr_layout = get_layout_of(
            &self.types.resolve(ptr_ty),
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if ptr_layout.size > 0 {
            self.push_instruction(Instruction::Memory(MemoryInstr::StackAlloc {
                dest,
                count,
            }));
        }

        dest
    }

    pub fn emit_heap_alloc(&mut self, value_type: TypeId, count: ValueId) -> ValueId {
        let ptr_ty = self.types.ptr(value_type);
        let dest = self.new_value_id(ptr_ty);

        let ptr_layout = get_layout_of(
            &self.types.resolve(ptr_ty),
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if ptr_layout.size > 0 {
            self.push_instruction(Instruction::Memory(MemoryInstr::HeapAlloc {
                dest,
                count,
            }));
        }

        dest
    }

    pub fn emit_heap_free(&mut self, ptr: ValueId) {
        let ptr_ty = self.get_value_type(ptr);

        let ptr_layout = get_layout_of(
            &self.types.resolve(ptr_ty),
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if ptr_layout.size > 0 {
            self.push_instruction(Instruction::Memory(MemoryInstr::HeapFree { ptr }));
        }
    }

    pub fn emit_load(&mut self, ptr: ValueId) -> ValueId {
        let ptr_ty = self.get_value_type(ptr);
        let dest_ty = self.types.unwrap_ptr(ptr_ty);

        let dest = self.new_value_id(dest_ty);

        let dest_type_resolved = self.types.resolve(dest_ty);
        let layout = get_layout_of(
            &dest_type_resolved,
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if layout.size > 0 {
            self.push_instruction(Instruction::Memory(MemoryInstr::Load { dest, ptr }));
        }

        dest
    }

    pub fn emit_memcopy(&mut self, src: ValueId, dest: ValueId) {
        let src_ptr_ty = self.get_value_type(src);
        let dest_ptr_ty = self.get_value_type(dest);

        let src_ty = self.types.unwrap_ptr(src_ptr_ty);
        let dest_ty = self.types.unwrap_ptr(dest_ptr_ty);

        if src_ty != dest_ty {
            panic!(
                "INTERNAL COMPILER ERROR: MemCopy expected source and destination to be \
                 pointers to the same inner type"
            );
        }

        let src_type_resolved = self.types.resolve(src_ty);
        let layout = get_layout_of(
            &src_type_resolved,
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if layout.size > 0 {
            self.push_instruction(Instruction::Memory(MemoryInstr::MemCopy {
                dest,
                src,
            }));
        }
    }

    pub fn emit_store(&mut self, ptr: ValueId, value: ValueId) {
        let ptr_ty = self.get_value_type(ptr);
        let value_type = self.get_value_type(value);
        let expected_type = self.types.unwrap_ptr(ptr_ty);

        if value_type != expected_type
            && value_type != self.types.unknown()
            && expected_type != self.types.unknown()
        {
            panic!(
                "INTERNAL COMPILER ERROR: Store instruction expected the provided value \
                 to match pointed to type"
            );
        }

        let value_type_resolved = self.types.resolve(value_type);
        let layout = get_layout_of(
            &value_type_resolved,
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if layout.size > 0 {
            self.push_instruction(Instruction::Memory(MemoryInstr::Store { ptr, value }));
        }
    }

    /// offset = ptr<T> + index * sizeof(T)
    pub fn ptr_offset(&mut self, base_ptr: ValueId, index: ValueId) -> ValueId {
        let ptr_ty = self.get_value_type(base_ptr);
        let index_ty = self.get_value_type(index);

        if !matches!(self.types.resolve(index_ty), Type::USize) {
            panic!(
                "INTERNAL COMPILER ERROR: ptr_offset requires the index to be of type \
                 usize"
            );
        }

        if let Type::Pointer(to) = self.types.resolve(ptr_ty) {
            let to_resolved = self.types.resolve(to);
            let layout = get_layout_of(
                &to_resolved,
                self.types,
                self.program.target_ptr_size,
                self.program.target_ptr_align,
            );

            if layout.size == 0 {
                return base_ptr;
            }

            let dest_ty = self.types.ptr(to);
            let dest = self.new_value_id(dest_ty);

            self.push_instruction(Instruction::Memory(MemoryInstr::PtrOffset {
                dest,
                base_ptr,
                index,
            }));

            dest
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: Memory offset expected base to be a pointer"
            );
        }
    }

    pub fn try_get_field_ptr(
        &mut self,
        base_ptr: ValueId,
        field: &IdentifierNode,
        is_internal_access: bool,
    ) -> Result<ValueId, SemanticError> {
        let base_ptr_ty = self.get_value_type(base_ptr);
        let actual_base_ptr_ty = self.types.unwrap_generic_bound(base_ptr_ty);
        let pointee_ty = self.types.unwrap_ptr(actual_base_ptr_ty);
        let actual_pointee_ty = self.types.unwrap_generic_bound(pointee_ty);

        let struct_kind = match self.types.resolve(actual_pointee_ty) {
            Type::Struct(s) => {
                if is_internal_access {
                    s
                } else if let StructKind::UserDefined(_) = s {
                    s
                } else {
                    panic!(
                        "INTERNAL COMPILER ERROR: Compiler allowed the user to attempt \
                         access to non-user space struct field"
                    )
                }
            }
            _ => panic!("Expected pointer to struct, found {:?}", pointee_ty),
        };

        let (field_index, field_type) =
            if let Some(v) = struct_kind.get_field(self.types, &field.name) {
                v
            } else {
                return Err(SemanticError {
                    span: field.span.clone(),
                    kind: SemanticErrorKind::AccessToUndefinedField(field.clone()),
                });
            };

        let field_ptr_ty = self.types.ptr(field_type);
        let dest = self.new_value_id(field_ptr_ty);

        let ptr_layout = get_layout_of(
            &self.types.resolve(field_ptr_ty),
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if ptr_layout.size > 0 {
            self.push_instruction(Instruction::Memory(MemoryInstr::GetFieldPtr {
                dest,
                base_ptr,
                field_index,
            }));
        }

        Ok(dest)
    }

    pub fn get_field_ptr(&mut self, base_ptr: ValueId, field_name: StringId) -> ValueId {
        self.try_get_field_ptr(
            base_ptr,
            &IdentifierNode {
                name: field_name,
                span: Span::default(),
            },
            true,
        )
        .unwrap_or_else(|_| {
            panic!(
                "INTERNAL COMPILER ERROR: Expected field {} to be defined",
                STRING_INTERNER.resolve(field_name)
            )
        })
    }

    pub fn get_list_buffer_ptr(&mut self, list_header_ptr: ValueId) -> ValueId {
        let list_ptr_type = self.get_value_type(list_header_ptr);

        let is_valid = if let Type::Pointer(inner) = self.types.resolve(list_ptr_type) {
            let inner_ty = self.types.resolve(inner);
            matches!(inner_ty, Type::Struct(StructKind::ListHeader(_)))
        } else {
            false
        };

        if !is_valid {
            panic!(
                "INTERNAL COMPILER ERROR: Tried to get list buffer pointer from an \
                 invalid header type"
            );
        }

        let buffer_ptr_ptr = self.get_field_ptr(list_header_ptr, COMMON_IDENTIFIERS.ptr);

        self.emit_load(buffer_ptr_ptr)
    }
}
