use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock},
        instructions::{
            BuiltinFunction, InstrId, InstructionKind, ListInitInstr, MakeLiteralKind,
        },
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_template_expr(&mut self, parts: Vec<Expr>, span: Span) -> InstrId {
        let mut evaluated_parts = Vec::with_capacity(parts.len());
        for part in &parts {
            evaluated_parts.push(self.build_expr(part.clone()));
        }

        let parts_len = evaluated_parts.len();
        let list_id = self.push_instruction(
            InstructionKind::ListInit(ListInitInstr {
                items: evaluated_parts,
            }),
            span.clone(),
        );

        let count_id = self.push_instruction(
            InstructionKind::MakeLiteral(MakeLiteralKind::Number(NumberKind::USize(
                parts_len,
            ))),
            span.clone(),
        );

        self.emit_call_builtin(
            BuiltinFunction::StringConcat,
            vec![list_id, count_id],
            span,
        )
    }
}
