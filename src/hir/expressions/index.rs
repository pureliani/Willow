use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{InstrId, InstructionKind, MemoryInstr, Place},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_index_expr(&mut self, left: Expr, index: Expr, span: Span) -> InstrId {
        let base_place = match self.build_place(left.clone()) {
            Ok(p) => p,
            Err(e) => {
                self.build_expr(index);
                return self.report_error_and_get_poison(e);
            }
        };

        let index_id = self.build_expr(index);
        let full_place = Place::Index(Box::new(base_place), index_id);

        let memory_in = self.read_memory(self.context.block_id);
        self.push_instruction(
            InstructionKind::Memory(MemoryInstr::ReadPlace {
                place: full_place,
                memory_in,
            }),
            span,
        )
    }
}
