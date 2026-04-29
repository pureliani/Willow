use crate::{
    ast::expr::Expr,
    hir::{
        builders::{Builder, InBlock},
        instructions::{
            BinaryInstr, BinaryOpKind, InstrDefinition, InstrId, InstructionKind,
        },
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_binary(&mut self, left: Expr, right: Expr, op: BinaryOpKind) -> InstrId {
        let left_span = left.span.clone();
        let left_value = self.build_expr(left);
        let right_span = right.span.clone();
        let right_value = self.build_expr(right);

        let instr = InstrDefinition {
            block: self.context.block_id,
            span: left_span.merge(&right_span),
            kind: InstructionKind::Binary(BinaryInstr {
                lhs: left_value,
                rhs: right_value,
                op,
            }),
        };

        self.push_instruction(instr)
    }
}
