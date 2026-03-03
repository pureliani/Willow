use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{Instruction, UnaryInstr},
        types::checked_type::Type,
        utils::numeric::is_signed,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_neg(&mut self, src: ValueId, span: Span) -> ValueId {
        let ty = self.get_value_type(src);

        if !is_signed(ty) {
            return self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::ExpectedASignedNumericOperand,
                span,
            });
        }

        let dest = self.new_value_id(ty.clone());
        self.push_instruction(Instruction::Unary(UnaryInstr::Neg { dest, src }));
        dest
    }

    pub fn emit_not(&mut self, src: ValueId, span: Span) -> ValueId {
        let ty = self.get_value_type(src);

        if ty != &Type::Bool {
            return self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    expected: Type::Bool,
                    received: ty.clone(),
                },
                span,
            });
        }

        let dest = self.new_value_id(Type::Bool);
        self.push_instruction(Instruction::Unary(UnaryInstr::Not { dest, src }));
        dest
    }
}
