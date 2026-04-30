use crate::{
    ast::Span,
    hir::{
        builders::{BasicBlockId, Builder, InBlock},
        instructions::{
            CallInstr, InstrDefinition, InstrId, InstructionKind, MakeLiteralKind,
            PhiInstr, PhiSource, Terminator,
        },
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn push_instruction(&mut self, kind: InstructionKind, span: Span) -> InstrId {
        let block = self.context.block_id;
        let def = InstrDefinition { kind, block, span };
        self.cfg_mut().push_instruction(def)
    }

    pub fn check_no_terminator(&self) {
        let block = self.context.block_id;
        self.cfg().check_no_terminator(block);
    }

    pub fn emit_call(
        &mut self,
        func: InstrId,
        args: Vec<InstrId>,
        span: Span,
    ) -> InstrId {
        let memory_in = self.read_memory(self.context.block_id);
        let memory_out = self.cfg_mut().new_memory_id();
        self.write_memory(self.context.block_id, memory_out);

        self.push_instruction(
            InstructionKind::Call(CallInstr {
                func,
                args,
                memory_in,
                memory_out,
            }),
            span,
        )
    }

    pub fn emit_jmp(&mut self, target: BasicBlockId) {
        self.check_no_terminator();
        let this_block_id = self.context.block_id;
        self.get_bb_mut(target).predecessors.insert(this_block_id);

        self.bb_mut().terminator = Some(Terminator::Jump { target });
    }

    pub fn emit_cond_jmp(
        &mut self,
        condition: InstrId,
        true_target: BasicBlockId,
        false_target: BasicBlockId,
    ) {
        self.check_no_terminator();
        let this_block_id = self.context.block_id;

        self.get_bb_mut(true_target)
            .predecessors
            .insert(this_block_id);
        self.get_bb_mut(false_target)
            .predecessors
            .insert(this_block_id);

        self.bb_mut().terminator = Some(Terminator::CondJump {
            condition,
            true_target,
            false_target,
        });
    }

    pub fn emit_return(&mut self, value: InstrId) {
        self.check_no_terminator();
        self.bb_mut().terminator = Some(Terminator::Return { value })
    }

    pub fn emit_unreachable(&mut self) {
        self.check_no_terminator();
        self.bb_mut().terminator = Some(Terminator::Unreachable)
    }

    pub fn emit_logical_or<F>(
        &mut self,
        left: InstrId,
        span: Span,
        produce_right: F,
    ) -> InstrId
    where
        F: FnOnce(&mut Self) -> InstrId,
    {
        let left_block = self.context.block_id;

        let right_entry_block = self.new_block();
        let merge_block = self.new_block();

        let const_true = self.push_instruction(
            InstructionKind::MakeLiteral(MakeLiteralKind::Bool(true)),
            span.clone(),
        );

        self.emit_cond_jmp(left, merge_block, right_entry_block);

        self.use_basic_block(right_entry_block);
        self.seal_block(right_entry_block);

        let right = produce_right(self);
        let right_end_block = self.context.block_id;
        self.emit_jmp(merge_block);

        self.use_basic_block(merge_block);

        self.seal_block(merge_block);

        let phi_instr = InstructionKind::Phi(PhiInstr {
            sources: vec![
                PhiSource {
                    block: left_block,
                    value: const_true,
                },
                PhiSource {
                    block: right_end_block,
                    value: right,
                },
            ],
        });

        self.push_instruction(phi_instr, span)
    }

    pub fn emit_logical_and<F>(
        &mut self,
        left: InstrId,
        span: Span,
        produce_right: F,
    ) -> InstrId
    where
        F: FnOnce(&mut Self) -> InstrId,
    {
        let left_block = self.context.block_id;

        let right_entry_block = self.new_block();
        let merge_block = self.new_block();

        let const_false = self.push_instruction(
            InstructionKind::MakeLiteral(MakeLiteralKind::Bool(false)),
            span.clone(),
        );

        self.emit_cond_jmp(left, right_entry_block, merge_block);

        self.use_basic_block(right_entry_block);
        self.seal_block(right_entry_block);

        let right = produce_right(self);
        let right_end_block = self.context.block_id;
        self.emit_jmp(merge_block);

        self.use_basic_block(merge_block);
        self.seal_block(merge_block);

        let phi_instr = InstructionKind::Phi(PhiInstr {
            sources: vec![
                PhiSource {
                    block: left_block,
                    value: const_false,
                },
                PhiSource {
                    block: right_end_block,
                    value: right,
                },
            ],
        });

        self.push_instruction(phi_instr, span)
    }
}
