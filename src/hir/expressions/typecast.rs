use crate::{
    ast::{expr::Expr, type_annotation::TypeAnnotation, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{CastInstr, InstrId, InstructionKind},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_typecast_expr(
        &mut self,
        left: Expr,
        target: TypeAnnotation,
        span: Span,
    ) -> InstrId {
        let src = self.build_expr(left);
        self.push_instruction(InstructionKind::Cast(CastInstr { src, target }), span)
    }
}
