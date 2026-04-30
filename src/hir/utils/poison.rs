use crate::hir::{
    builders::{Builder, InBlock},
    errors::SemanticError,
    instructions::{InstrDefinition, InstrId, InstructionKind, MakeLiteralKind},
};

impl<'a> Builder<'a, InBlock> {
    pub fn report_error_and_get_poison(&mut self, error: SemanticError) -> InstrId {
        let span = error.span.clone();
        self.errors.push(error);
        self.push_instruction(InstrDefinition {
            span,
            block: self.context.block_id,
            kind: InstructionKind::MakeLiteral(MakeLiteralKind::Unknown),
        })
    }
}
