use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{InstrId, InstructionKind, UnaryInstr, UnaryOpKind},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_unary(&mut self, span: Span, value: Expr, op: UnaryOpKind) -> InstrId {
        let value = self.build_expr(value);
        self.push_instruction(InstructionKind::Unary(UnaryInstr { value, op }), span)
    }
}
