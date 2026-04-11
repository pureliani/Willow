use crate::{
    ast::{expr::BlockContents, Span},
    compile::interner::GenericSubstitutions,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::SpannedType,
        utils::scope::ScopeKind,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_codeblock_expr(
        &mut self,
        codeblock: BlockContents,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> (ValueId, Span) {
        let mut final_expr_span = Span {
            start: codeblock.span.end,
            end: codeblock.span.end,
            path: codeblock.span.path.clone(),
        };

        self.current_scope = self
            .current_scope
            .enter(ScopeKind::CodeBlock, codeblock.span.start);

        self.build_statements(codeblock.statements, substitutions);
        let result_id = if let Some(final_expr) = codeblock.final_expr {
            final_expr_span = final_expr.span.clone();
            self.build_expr(*final_expr, expected_type, substitutions)
        } else {
            self.emit_void()
        };

        self.current_scope = self
            .current_scope
            .exit(codeblock.span.end)
            .expect("INTERNAL COMPILER ERROR: Scope stack mismatch in codeblock");

        (result_id, final_expr_span)
    }
}
