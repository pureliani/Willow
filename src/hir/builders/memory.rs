use crate::{
    ast::{type_annotation::TypeAnnotation, IdentifierNode, Span},
    hir::{
        builders::{Builder, ExpectBody, InBlock},
        instructions::{InstrId, InstructionKind, MemoryInstr},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_stack_alloc(
        &mut self,
        ty: TypeAnnotation,
        count: usize,
        span: Span,
    ) -> InstrId {
        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::StackAlloc { ty, count }),
            span,
        )
    }

    pub fn emit_heap_alloc(
        &mut self,
        ty: TypeAnnotation,
        count: InstrId,
        span: Span,
    ) -> InstrId {
        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::HeapAlloc { ty, count }),
            span,
        )
    }

    pub fn emit_heap_free(&mut self, ptr: InstrId, span: Span) -> InstrId {
        let memory_in = self.read_memory(self.context.block_id);
        let memory_out = self.get_fn_mut().expect_body().new_memory_id();
        self.write_memory(self.context.block_id, memory_out);

        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::HeapFree {
                ptr,
                memory_in,
                memory_out,
            }),
            span,
        )
    }

    pub fn emit_load(&mut self, ptr: InstrId, span: Span) -> InstrId {
        let memory_in = self.read_memory(self.context.block_id);

        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::Load { ptr, memory_in }),
            span,
        )
    }

    pub fn emit_store(&mut self, ptr: InstrId, value: InstrId, span: Span) -> InstrId {
        let memory_in = self.read_memory(self.context.block_id);
        let memory_out = self.get_fn_mut().expect_body().new_memory_id();
        self.write_memory(self.context.block_id, memory_out);

        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::Store {
                ptr,
                value,
                memory_in,
                memory_out,
            }),
            span,
        )
    }

    pub fn emit_memcopy(&mut self, from: InstrId, to: InstrId, span: Span) -> InstrId {
        let memory_in = self.read_memory(self.context.block_id);
        let memory_out = self.get_fn_mut().expect_body().new_memory_id();
        self.write_memory(self.context.block_id, memory_out);

        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::MemCopy {
                from,
                to,
                memory_in,
                memory_out,
            }),
            span,
        )
    }

    pub fn emit_get_field_ptr(
        &mut self,
        base_ptr: InstrId,
        field_name: IdentifierNode,
        span: Span,
    ) -> InstrId {
        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::GetFieldPtr {
                base_ptr,
                field_name,
            }),
            span,
        )
    }

    pub fn emit_ptr_offset(
        &mut self,
        base_ptr: InstrId,
        index: InstrId,
        span: Span,
    ) -> InstrId {
        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::PtrOffset { base_ptr, index }),
            span,
        )
    }
}
