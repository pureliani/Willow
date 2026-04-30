use crate::{
    ast::expr::{BlockContents, Expr},
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{InstrId, InstructionKind, MakeLiteralKind, PhiInstr, PhiSource},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IfContext {
    Expression,
    Statement,
}

impl<'a> Builder<'a, InBlock> {
    pub fn build_if(
        &mut self,
        branches: Vec<(Box<Expr>, BlockContents)>,
        else_branch: Option<BlockContents>,
        context: IfContext,
    ) -> InstrId {
        let expr_span = branches.first().unwrap().0.span.clone();

        if context == IfContext::Expression && else_branch.is_none() {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::IfExpressionMissingElse,
                span: expr_span.clone(),
            });
        }

        let merge_block_id = self.new_block();
        let mut phi_sources: Vec<PhiSource> = Vec::new();
        let mut current_cond_block_id = self.context.block_id;

        for (condition, body) in branches {
            self.use_basic_block(current_cond_block_id);

            let cond_id = self.build_expr(*condition);

            let then_block_id = self.new_block();
            let next_cond_block_id = self.new_block();

            self.emit_cond_jmp(cond_id, then_block_id, next_cond_block_id);

            self.use_basic_block(then_block_id);
            self.seal_block(then_block_id);

            let (then_val, _) = self.build_codeblock_expr(body);

            if self.bb().terminator.is_none() {
                self.emit_jmp(merge_block_id);
                phi_sources.push(PhiSource {
                    block: self.context.block_id,
                    value: then_val,
                });
            }

            self.use_basic_block(next_cond_block_id);
            self.seal_block(next_cond_block_id);
            current_cond_block_id = next_cond_block_id;
        }

        self.use_basic_block(current_cond_block_id);

        if let Some(else_body) = else_branch {
            let (else_val, _) = self.build_codeblock_expr(else_body);

            if self.bb().terminator.is_none() {
                self.emit_jmp(merge_block_id);
                phi_sources.push(PhiSource {
                    block: self.context.block_id,
                    value: else_val,
                });
            }
        } else {
            if self.bb().terminator.is_none() {
                self.emit_jmp(merge_block_id);

                let fallback_val = if context == IfContext::Expression {
                    self.push_instruction(
                        InstructionKind::MakeLiteral(MakeLiteralKind::Unknown),
                        expr_span.clone(),
                    )
                } else {
                    self.push_instruction(
                        InstructionKind::MakeLiteral(MakeLiteralKind::Void),
                        expr_span.clone(),
                    )
                };

                phi_sources.push(PhiSource {
                    block: self.context.block_id,
                    value: fallback_val,
                });
            }
        }

        self.use_basic_block(merge_block_id);

        self.seal_block(merge_block_id);

        if context == IfContext::Expression {
            if phi_sources.is_empty() {
                return self.push_instruction(
                    InstructionKind::MakeLiteral(MakeLiteralKind::Never),
                    expr_span,
                );
            }

            self.push_instruction(
                InstructionKind::Phi(PhiInstr {
                    sources: phi_sources,
                }),
                expr_span,
            )
        } else {
            self.push_instruction(
                InstructionKind::MakeLiteral(MakeLiteralKind::Void),
                expr_span,
            )
        }
    }
}
