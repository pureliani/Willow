use crate::{
    ast::{expr::Expr, type_annotation::TypeAnnotation, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{GenericApplyInstr, InstrId, InstructionKind},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_generic_apply_expr(
        &mut self,
        left: Expr,
        type_args: Vec<TypeAnnotation>,
        span: Span,
    ) -> InstrId {
        let func = self.build_expr(left);
        self.push_instruction(
            InstructionKind::GenericApply(GenericApplyInstr { func, type_args }),
            span,
        )
    }
}
