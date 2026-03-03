use std::collections::HashSet;

use crate::{
    ast::Span,
    hir::{
        builders::{BasicBlockId, Builder, InBlock, PhiSource, TypePredicate, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{CallInstr, Instruction, Terminator},
        types::checked_type::Type,
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
        return_type: Type,
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

    pub fn emit_logical_or<F>(
        &mut self,
        left: ValueId,
        left_span: Span,
        right_span: Span,
        produce_right: F,
    ) -> ValueId
    where
        F: FnOnce(&mut Self) -> ValueId,
    {
        let left_type = self.get_value_type(left);
        if left_type != &Type::Bool {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    expected: Type::Bool,
                    received: left_type.clone(),
                },
                span: left_span.clone(),
            });
        }

        let left_preds = self.type_predicates.get(&left).cloned().unwrap_or_default();

        let left_block = self.context.block_id;
        let right_entry_block = self.as_fn().new_bb();
        let merge_block = self.as_fn().new_bb();

        let const_true = self.emit_const_bool(true);

        self.emit_cond_jmp(left, merge_block, right_entry_block);

        self.seal_block(right_entry_block);
        self.use_basic_block(right_entry_block);

        self.apply_predicate_list(&left_preds, false, &left_span);

        let right = produce_right(self);
        let right_block = self.context.block_id;

        let right_type = self.get_value_type(right);
        if right_type != &Type::Bool {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    expected: Type::Bool,
                    received: right_type.clone(),
                },
                span: right_span,
            });
        }

        let right_preds = self
            .type_predicates
            .get(&right)
            .cloned()
            .unwrap_or_default();

        self.emit_jmp(merge_block);

        self.seal_block(merge_block);
        self.use_basic_block(merge_block);

        let phi_id = self.new_value_id(Type::Bool);
        let phi_sources = HashSet::from([
            PhiSource {
                from: left_block,
                value: const_true,
            },
            PhiSource {
                from: right_block,
                value: right,
            },
        ]);

        self.insert_phi(self.context.block_id, phi_id, phi_sources);

        let combined = Self::combine_predicates(&left_preds, &right_preds, false);
        if !combined.is_empty() {
            self.type_predicates.insert(phi_id, combined);
        }

        phi_id
    }

    pub fn emit_logical_and<F>(
        &mut self,
        left: ValueId,
        left_span: Span,
        right_span: Span,
        produce_right: F,
    ) -> ValueId
    where
        F: FnOnce(&mut Self) -> ValueId,
    {
        let left_type = self.get_value_type(left);
        if left_type != &Type::Bool {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    expected: Type::Bool,
                    received: left_type.clone(),
                },
                span: left_span.clone(),
            });
        }

        let left_preds = self.type_predicates.get(&left).cloned().unwrap_or_default();

        let left_block = self.context.block_id;
        let right_entry_block = self.as_fn().new_bb();
        let merge_block = self.as_fn().new_bb();

        let const_false = self.emit_const_bool(false);

        self.emit_cond_jmp(left, right_entry_block, merge_block);

        self.seal_block(right_entry_block);
        self.use_basic_block(right_entry_block);

        self.apply_predicate_list(&left_preds, true, &left_span);

        let right = produce_right(self);
        let right_block = self.context.block_id;

        let right_type = self.get_value_type(right);
        if right_type != &Type::Bool {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    expected: Type::Bool,
                    received: right_type.clone(),
                },
                span: right_span,
            });
        }

        let right_preds = self
            .type_predicates
            .get(&right)
            .cloned()
            .unwrap_or_default();

        self.emit_jmp(merge_block);

        self.seal_block(merge_block);
        self.use_basic_block(merge_block);

        let phi_id = self.new_value_id(Type::Bool);
        let phi_sources = HashSet::from([
            PhiSource {
                from: left_block,
                value: const_false,
            },
            PhiSource {
                from: right_block,
                value: right,
            },
        ]);

        self.insert_phi(self.context.block_id, phi_id, phi_sources);

        let combined = Self::combine_predicates(&left_preds, &right_preds, true);
        if !combined.is_empty() {
            self.type_predicates.insert(phi_id, combined);
        }

        phi_id
    }

    fn combine_predicates(
        left_preds: &[TypePredicate],
        right_preds: &[TypePredicate],
        keep_true_side: bool,
    ) -> Vec<TypePredicate> {
        let mut combined = Vec::new();

        for pred in left_preds.iter().chain(right_preds.iter()) {
            let (on_true, on_false) = if keep_true_side {
                (pred.on_true_type.clone(), None)
            } else {
                (None, pred.on_false_type.clone())
            };

            if on_true.is_some() || on_false.is_some() {
                combined.push(TypePredicate {
                    decl_id: pred.decl_id,
                    on_true_type: on_true,
                    on_false_type: on_false,
                });
            }
        }

        combined
    }
}
