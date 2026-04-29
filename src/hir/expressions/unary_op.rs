use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{
            InstrDefinition, InstrId, InstructionKind, UnaryInstr, UnaryOpKind,
        },
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_unary(&mut self, span: Span, value: Expr, op: UnaryOpKind) -> InstrId {
        let value = self.build_expr(value);

        let instr = InstrDefinition {
            block: self.context.block_id,
            span,
            kind: InstructionKind::Unary(UnaryInstr { value, op }),
        };

        self.push_instruction(instr)
    }
}
