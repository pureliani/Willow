use crate::{
    ast::expr::{BlockContents, Expr},
    mir::{
        builders::{Builder, InBlock},
        types::checked_type::{SpannedType, Type},
        utils::scope::ScopeKind,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_while_stmt(&mut self, condition: Expr, body: BlockContents) {
        let header_block_id = self.as_fn().new_bb();
        let body_block_id = self.as_fn().new_bb();
        let exit_block_id = self.as_fn().new_bb();

        self.emit_jmp(header_block_id);
        self.use_basic_block(header_block_id);

        let condition_span = condition.span.clone();
        let cond_id = self.build_expr(
            condition,
            Some(&SpannedType {
                kind: Type::Bool(None),
                span: condition_span,
            }),
        );

        self.emit_cond_jmp(cond_id, body_block_id, exit_block_id);

        self.seal_block(body_block_id);
        self.use_basic_block(body_block_id);

        self.current_scope = self.current_scope.enter(
            ScopeKind::WhileBody {
                break_target: exit_block_id,
                continue_target: header_block_id,
            },
            body.span.start,
        );

        self.build_statements(body.statements);
        if let Some(final_expr) = body.final_expr {
            self.build_expr(*final_expr, None);
        }

        self.current_scope = self
            .current_scope
            .exit(body.span.end)
            .expect("INTERNAL COMPILER ERROR: Scope mismatch");

        if self.bb().terminator.is_none() {
            self.emit_jmp(header_block_id);
        }

        self.seal_block(header_block_id);

        self.use_basic_block(exit_block_id);
        self.seal_block(exit_block_id);
    }
}
