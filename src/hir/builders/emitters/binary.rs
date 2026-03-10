use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{BinaryInstr, Instruction},
        utils::adjustment::{arithmetic_supertype, compute_type_adjustment},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_arithmetic_op<OP: FnOnce(ValueId, ValueId, ValueId) -> BinaryInstr>(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
        op: OP,
    ) -> ValueId {
        let lhs_type = self.get_value_type(lhs).clone();
        let rhs_type = self.get_value_type(rhs).clone();

        let result_type = match arithmetic_supertype(
            &lhs_type,
            lhs_span.clone(),
            &rhs_type,
            rhs_span.clone(),
        ) {
            Ok(t) => t,
            Err(e) => return self.report_error_and_get_poison(e),
        };

        let left_adjustment = compute_type_adjustment(&lhs_type, &result_type, false);
        let adjusted_lhs = match left_adjustment {
            Ok(adj) => self.apply_adjustment(lhs, adj, result_type.clone()),
            Err(_) => {
                return self.report_error_and_get_poison(SemanticError {
                    kind: SemanticErrorKind::TypeMismatch {
                        expected: result_type,
                        received: lhs_type.clone(),
                    },
                    span: lhs_span,
                })
            }
        };

        let right_adjustment = compute_type_adjustment(&rhs_type, &result_type, false);
        let adjusted_rhs = match right_adjustment {
            Ok(adj) => self.apply_adjustment(rhs, adj, result_type.clone()),
            Err(_) => {
                return self.report_error_and_get_poison(SemanticError {
                    kind: SemanticErrorKind::TypeMismatch {
                        expected: result_type.clone(),
                        received: rhs_type.clone(),
                    },
                    span: rhs_span,
                })
            }
        };

        let dest = self.new_value_id(result_type);
        let binary_instr = op(dest, adjusted_lhs, adjusted_rhs);

        self.push_instruction(Instruction::Binary(binary_instr));
        dest
    }

    pub fn emit_add(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_arithmetic_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            BinaryInstr::Add { dest, lhs, rhs }
        })
    }

    pub fn emit_sub(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_arithmetic_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            BinaryInstr::Sub { dest, lhs, rhs }
        })
    }

    pub fn emit_mul(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_arithmetic_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            BinaryInstr::Mul { dest, lhs, rhs }
        })
    }

    pub fn emit_div(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_arithmetic_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            BinaryInstr::Div { dest, lhs, rhs }
        })
    }

    pub fn emit_rem(
        &mut self,
        lhs: ValueId,
        lhs_span: Span,
        rhs: ValueId,
        rhs_span: Span,
    ) -> ValueId {
        self.emit_arithmetic_op(lhs, lhs_span, rhs, rhs_span, |dest, lhs, rhs| {
            BinaryInstr::Rem { dest, lhs, rhs }
        })
    }
}
