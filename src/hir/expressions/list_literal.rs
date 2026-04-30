use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{InstrId, InstructionKind, ListInitInstr},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_list_literal_expr(&mut self, span: Span, items: Vec<Expr>) -> InstrId {
        let mut evaluated_items = Vec::with_capacity(items.len());

        for item in items {
            let val_id = self.build_expr(item);
            evaluated_items.push(val_id);
        }

        self.push_instruction(
            InstructionKind::ListInit(ListInitInstr {
                items: evaluated_items,
            }),
            span,
        )
    }
}
