use crate::{
    ast::expr::Expr,
    hir::{
        builders::{Builder, InBlock},
        instructions::{BinaryInstr, BinaryOpKind, InstrId, InstructionKind},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_binary(&mut self, left: Expr, right: Expr, op: BinaryOpKind) -> InstrId {
        let left_span = left.span.clone();
        let left_value = self.build_expr(left);
        let right_span = right.span.clone();
        let right_value = self.build_expr(right);

        self.push_instruction(
            InstructionKind::Binary(BinaryInstr {
                lhs: left_value,
                rhs: right_value,
                op,
            }),
            left_span.merge(&right_span),
        )
    }
}
