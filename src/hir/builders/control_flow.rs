use crate::{
    ast::Span,
    compile::interner::TypeId,
    hir::{
        builders::{BasicBlockId, Builder, ConditionFact, InBlock, ValueId},
        instructions::{CallInstr, Instruction, Terminator},
        types::checked_type::LiteralType,
        utils::facts::FactSet,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn push_instruction(&mut self, instruction: Instruction) {
        self.check_no_terminator();
        let bb = self.bb_mut();
        bb.instructions.push(instruction);
    }

    pub fn check_no_terminator(&mut self) {
        let bb = self.bb_mut();

        if bb.terminator.is_some() {
            panic!(
                "INTERNAL COMPILER ERROR: Tried re-set terminator or tried to add an \
                 instruction to a basic block (ID: {}) that has already been terminated",
                bb.id.0
            );
        }
    }

    pub fn emit_call(
        &mut self,
        func: ValueId,
        args: Vec<ValueId>,
        return_type: TypeId,
    ) -> ValueId {
        let dest = self.new_value_id(return_type);
        self.push_instruction(Instruction::Call(CallInstr { dest, func, args }));
        dest
    }

    pub fn emit_jmp(&mut self, target: BasicBlockId) {
        self.check_no_terminator();
        let this_block_id = self.context.block_id;
        self.get_bb_mut(target).predecessors.insert(this_block_id);

        self.bb_mut().terminator = Some(Terminator::Jump { target });
    }

    pub fn emit_cond_jmp(
        &mut self,
        condition: ValueId,
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

    pub fn emit_return(&mut self, value: ValueId) {
        self.check_no_terminator();
        self.bb_mut().terminator = Some(Terminator::Return { value })
    }

    pub fn emit_unreachable(&mut self) {
        self.check_no_terminator();
        self.bb_mut().terminator = Some(Terminator::Unreachable)
    }

    pub fn emit_logical_or<F>(
        &mut self,
        left: ValueId,
        _left_span: Span,
        produce_right: F,
    ) -> ValueId
    where
        F: FnOnce(&mut Self) -> ValueId,
    {
        let left_facts = self.condition_facts.get(&left).cloned().unwrap_or_default();

        let right_entry_block = self.as_fn().new_bb();
        let merge_block = self.as_fn().new_bb();

        let bool_ty = self.types.bool(None);
        let result_ptr = self.emit_stack_alloc(bool_ty, 1);

        let const_true = self.emit_materialize(LiteralType::Bool(true));
        self.emit_store(result_ptr, const_true);

        self.emit_cond_jmp(left, merge_block, right_entry_block);

        self.seal_block(right_entry_block);
        self.use_basic_block(right_entry_block);

        for fact in &left_facts {
            if !fact.on_false.facts.is_empty() {
                self.write_fact(right_entry_block, &fact.place, fact.on_false.clone());
            }
        }

        let right = produce_right(self);

        let right_facts = self
            .condition_facts
            .get(&right)
            .cloned()
            .unwrap_or_default();

        self.emit_store(result_ptr, right);
        self.emit_jmp(merge_block);

        self.seal_block(merge_block);
        self.use_basic_block(merge_block);

        let result_val = self.emit_load(result_ptr);

        let combined = Self::combine_condition_facts(&left_facts, &right_facts, false);
        if !combined.is_empty() {
            self.condition_facts.insert(result_val, combined);
        }

        result_val
    }

    pub fn emit_logical_and<F>(
        &mut self,
        left: ValueId,
        _left_span: Span,
        produce_right: F,
    ) -> ValueId
    where
        F: FnOnce(&mut Self) -> ValueId,
    {
        let left_facts = self.condition_facts.get(&left).cloned().unwrap_or_default();

        let right_entry_block = self.as_fn().new_bb();
        let merge_block = self.as_fn().new_bb();

        let bool_ty = self.types.bool(None);
        let result_ptr = self.emit_stack_alloc(bool_ty, 1);

        let const_false = self.emit_materialize(LiteralType::Bool(false)); // FIX
        self.emit_store(result_ptr, const_false);

        self.emit_cond_jmp(left, right_entry_block, merge_block);

        self.seal_block(right_entry_block);
        self.use_basic_block(right_entry_block);

        for fact in &left_facts {
            if !fact.on_true.facts.is_empty() {
                self.write_fact(right_entry_block, &fact.place, fact.on_true.clone());
            }
        }

        let right = produce_right(self);

        let right_facts = self
            .condition_facts
            .get(&right)
            .cloned()
            .unwrap_or_default();

        self.emit_store(result_ptr, right);
        self.emit_jmp(merge_block);

        self.seal_block(merge_block);
        self.use_basic_block(merge_block);

        let result_val = self.emit_load(result_ptr);

        let combined = Self::combine_condition_facts(&left_facts, &right_facts, true);
        if !combined.is_empty() {
            self.condition_facts.insert(result_val, combined);
        }

        result_val
    }

    fn combine_condition_facts(
        left_facts: &[ConditionFact],
        right_facts: &[ConditionFact],
        keep_true_side: bool,
    ) -> Vec<ConditionFact> {
        let mut combined = Vec::new();

        for fact in left_facts.iter().chain(right_facts.iter()) {
            let (on_true, on_false) = if keep_true_side {
                (fact.on_true.clone(), FactSet::new())
            } else {
                (FactSet::new(), fact.on_false.clone())
            };

            if !on_true.facts.is_empty() || !on_false.facts.is_empty() {
                combined.push(ConditionFact {
                    place: fact.place.clone(),
                    on_true,
                    on_false,
                });
            }
        }

        combined
    }
}
