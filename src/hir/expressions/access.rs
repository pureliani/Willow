use crate::{
    ast::{expr::Expr, IdentifierNode},
    hir::{
        builders::{Builder, InBlock},
        instructions::{InstrId, Place},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_access_expr(&mut self, left: Expr, field: IdentifierNode) -> InstrId {
        let span = field.span.clone();

        let base_place = match self.build_place(left.clone()) {
            Ok(p) => p,
            Err(e) => {
                return self.report_error_and_get_poison(e);
            }
        };

        let full_place = Place::Field(Box::new(base_place), field);
        self.emit_read_place(full_place, span)
    }
}
