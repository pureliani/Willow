use crate::{
    ast::{expr::Expr, type_annotation::TypeAnnotation, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{InstrId, InstructionKind, IsTypeInstr},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_is_type_expr(
        &mut self,
        left: Expr,
        target: TypeAnnotation,
        span: Span,
    ) -> InstrId {
        let src = self.build_expr(left);
        self.push_instruction(InstructionKind::IsType(IsTypeInstr { src, target }), span)
    }
}
