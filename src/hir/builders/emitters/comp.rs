use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        instructions::{CompInstr, Instruction, SelectInstr},
        types::checked_type::Type,
    },
};

impl<'a> Builder<'a, InBlock> {
    fn emit_comp_op<F>(
        &mut self,
        lhs: ValueId,
        _lhs_span: Span,
        rhs: ValueId,
        _rhs_span: Span,
        make_instr: F,
    ) -> ValueId
    where
        F: FnOnce(ValueId, ValueId, ValueId) -> CompInstr,
    {
        // TODO: validate that operation is allowed using lhs_span and rhs_span
        // e.g. check if types are comparable

        let dest = self.new_value_id(Type::Bool);
        self.push_instruction(Instruction::Comp(make_instr(dest, lhs, rhs)));
        dest
    }

    pub fn emit_eq(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_comp_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            CompInstr::Eq { dest, lhs, rhs }
        })
    }

    pub fn emit_neq(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_comp_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            CompInstr::Neq { dest, lhs, rhs }
        })
    }

    pub fn emit_lt(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_comp_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            CompInstr::Lt { dest, lhs, rhs }
        })
    }

    pub fn emit_lte(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_comp_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            CompInstr::Lte { dest, lhs, rhs }
        })
    }

    pub fn emit_gt(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_comp_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            CompInstr::Gt { dest, lhs, rhs }
        })
    }

    pub fn emit_gte(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_comp_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            CompInstr::Gte { dest, lhs, rhs }
        })
    }

    pub fn emit_select(
        &mut self,
        condition: ValueId,
        true_value: ValueId,
        false_value: ValueId,
    ) -> ValueId {
        let condition_type = self.get_value_type(condition);

        if condition_type != &Type::Bool {
            panic!(
                "INTERNAL COMPILER ERROR: Select instruction expected the condition to \
                 be a boolean value"
            );
        }

        let true_value_type = self.get_value_type(true_value);
        let false_value_type = self.get_value_type(false_value);

        if true_value_type != false_value_type {
            panic!(
                "INTERNAL COMPILER ERROR: Select instruction expected both operands to \
                 have the same type"
            );
        }

        let dest = self.new_value_id(true_value_type.clone());
        self.push_instruction(Instruction::Select(SelectInstr {
            dest,
            cond: condition,
            true_val: true_value,
            false_val: false_value,
        }));

        dest
    }
}
