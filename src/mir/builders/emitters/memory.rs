use crate::{
    ast::{IdentifierNode, Span},
    compile::interner::StringId,
    globals::{COMMON_IDENTIFIERS, STRING_INTERNER},
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{Instruction, MemoryInstr},
        types::checked_type::{StructKind, Type},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_stack_alloc(&mut self, ty: Type, count: usize) -> ValueId {
        let dest = self.new_value_id(Type::Pointer(Box::new(ty)));
        self.push_instruction(Instruction::Memory(MemoryInstr::StackAlloc {
            dest,
            count,
        }));
        dest
    }

    pub fn emit_heap_alloc(&mut self, ty: Type, count: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Pointer(Box::new(ty)));
        self.push_instruction(Instruction::Memory(MemoryInstr::HeapAlloc {
            dest,
            count,
        }));
        dest
    }

    pub fn emit_load(&mut self, ptr: ValueId) -> ValueId {
        let ptr_ty = self.get_value_type(ptr);
        let dest_ty = if let Type::Pointer(to) = ptr_ty {
            *to.clone()
        } else {
            panic!("INTERNAL COMPILER ERROR: Load expected pointer");
        };

        let dest = self.new_value_id(dest_ty);
        self.push_instruction(Instruction::Memory(MemoryInstr::Load { dest, ptr }));
        dest
    }

    pub fn emit_memcopy(&mut self, src: ValueId, dest: ValueId) {
        let src_ty = self.get_value_type(src);
        let dest_ty = self.get_value_type(dest);

        let compatible = matches!((src_ty, dest_ty), (Type::Pointer(src_ptr), Type::Pointer(dest_ptr)) if src_ptr == dest_ptr);
        if !compatible {
            panic!(
                "INTERNAL COMPILER ERROR: MemCopy expected source and destination to be \
                 pointers to the same inner type"
            );
        }

        self.push_instruction(Instruction::Memory(MemoryInstr::MemCopy { dest, src }));
    }

    pub fn emit_store(&mut self, ptr: ValueId, value: ValueId) {
        let ptr_ty = self.get_value_type(ptr).clone();
        let val_ty = self.get_value_type(value).clone();

        if let Type::Pointer(to) = ptr_ty {
            if val_ty != *to {
                panic!("INTERNAL COMPILER ERROR: Store instruction expected the provided value to match pointed to type");
            }

            self.push_instruction(Instruction::Memory(MemoryInstr::Store { ptr, value }));
        } else {
            panic!("INTERNAL COMPILER ERROR: Store instruction expected a pointer");
        };
    }

    /// offset = ptr<T> + index * sizeof(T)
    pub fn ptr_offset(&mut self, base_ptr: ValueId, index: ValueId) -> ValueId {
        let ptr_ty = self.get_value_type(base_ptr).clone();
        let index_ty = self.get_value_type(index);

        if !matches!(index_ty, &Type::USize(_)) {
            panic!("INTERNAL COMPILER ERROR: ptr_offset requires the index to be of type usize");
        }

        if let Type::Pointer(to) = ptr_ty {
            let dest_ty = Type::Pointer(to);
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
        let current_ty = self.get_value_type(base_ptr);

        let struct_kind = match current_ty {
            Type::Pointer(to) => match &**to {
                Type::Struct(s) => {
                    if is_internal_access {
                        s
                    } else if let StructKind::UserDefined(_) = s {
                        s
                    } else {
                        panic!("INTERNAL COMPILER ERROR: Compiler allowed the user to attempt access to non-user space struct field")
                    }
                }
                _ => panic!("Expected pointer to struct, found {:?}", current_ty),
            },
            _ => {
                panic!("Expected pointer, found {:?}", current_ty);
            }
        };

        let (field_index, field_type) =
            if let Some(v) = struct_kind.get_field(&field.name) {
                v
            } else {
                return Err(SemanticError {
                    span: field.span.clone(),
                    kind: SemanticErrorKind::AccessToUndefinedField(field.clone()),
                });
            };

        let dest = self.new_value_id(Type::Pointer(Box::new(field_type)));

        self.push_instruction(Instruction::Memory(MemoryInstr::GetFieldPtr {
            dest,
            base_ptr,
            field_index,
        }));

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

        let is_valid = if let Type::Pointer(inner) = &list_ptr_type {
            matches!(
                &**inner,
                Type::Struct(StructKind::ListHeader(_))
                    | Type::Struct(StructKind::StringHeader(_))
            )
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
