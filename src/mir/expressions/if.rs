use crate::{
    ast::{
        expr::{BlockContents, Expr},
        Span,
    },
    compile::interner::TypeId,
    mir::{
        builders::{BasicBlockId, Builder, ExpectBody, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{Instruction, MemoryInstr},
        types::checked_type::{SpannedType, Type},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IfContext {
    /// The `if` is used to produce a value
    Expression,
    /// The `if` is used for control flow, its value is discarded
    Statement,
}

impl<'a> Builder<'a, InBlock> {
    pub fn build_if(
        &mut self,
        branches: Vec<(Box<Expr>, BlockContents)>,
        else_branch: Option<BlockContents>,
        context: IfContext,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let expr_span = branches.first().unwrap().0.span.clone();

        if context == IfContext::Expression && else_branch.is_none() {
            return self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::IfExpressionMissingElse,
                span: expr_span,
            });
        }

        let merge_block_id = self.as_fn().new_bb();
        let mut branch_results: Vec<(BasicBlockId, ValueId, Span)> = Vec::new();
        let mut current_cond_block_id = self.context.block_id;

        for (condition, body) in branches {
            self.use_basic_block(current_cond_block_id);

            let condition_span = condition.span.clone();
            let expected_condition_type = Some(SpannedType {
                id: self.types.bool(None),
                span: condition_span.clone(),
            });

            let cond_id = self.build_expr(*condition, expected_condition_type.as_ref());
            let cond_ty = self.get_value_type(cond_id);

            if cond_ty == self.types.unknown() {
                return self.new_value_id(self.types.unknown());
            }

            let then_block_id = self.as_fn().new_bb();
            let next_cond_block_id = self.as_fn().new_bb();

            self.emit_cond_jmp(cond_id, then_block_id, next_cond_block_id);

            self.seal_block(then_block_id);
            self.use_basic_block(then_block_id);

            if let Some(facts) = self.condition_facts.get(&cond_id).cloned() {
                for conditional_facts in &facts {
                    if !conditional_facts.on_true.facts.is_empty() {
                        self.write_fact(
                            then_block_id,
                            &conditional_facts.place,
                            conditional_facts.on_true.clone(),
                        );
                    }
                }
            }

            let (then_val, then_val_span) =
                self.build_codeblock_expr(body, expected_type, false);

            if self.bb().terminator.is_none() {
                branch_results.push((self.context.block_id, then_val, then_val_span));
            }

            self.use_basic_block(next_cond_block_id);

            if let Some(facts) = self.condition_facts.get(&cond_id).cloned() {
                for conditional_facts in &facts {
                    if !conditional_facts.on_false.facts.is_empty() {
                        self.write_fact(
                            next_cond_block_id,
                            &conditional_facts.place,
                            conditional_facts.on_false.clone(),
                        );
                    }
                }
            }

            current_cond_block_id = next_cond_block_id;
        }

        self.use_basic_block(current_cond_block_id);

        if let Some(else_body) = else_branch {
            let (else_val, else_val_span) =
                self.build_codeblock_expr(else_body, expected_type, false);

            if self.bb().terminator.is_none() {
                branch_results.push((self.context.block_id, else_val, else_val_span));
            }
        } else {
            if self.bb().terminator.is_none() {
                branch_results.push((
                    self.context.block_id,
                    self.emit_void(),
                    expr_span.clone(),
                ));
            }
        }

        let result = if context == IfContext::Expression {
            if branch_results.is_empty() {
                self.seal_block(merge_block_id);
                self.use_basic_block(merge_block_id);
                return self.new_value_id(self.types.intern(&Type::Never));
            }

            let type_entries: Vec<TypeId> = branch_results
                .iter()
                .map(|(_, val, _)| self.get_value_type(*val))
                .collect();

            let result_type = self.types.make_union(type_entries);

            let ptr_ty = self.types.ptr(result_type);
            let result_ptr = self.new_value_id(ptr_ty);
            let entry_block = self.get_fn().expect_body().entry_block;

            self.get_fn()
                .expect_body()
                .value_definitions
                .insert(result_ptr, entry_block);

            self.get_bb_mut(entry_block).instructions.insert(
                0,
                Instruction::Memory(MemoryInstr::StackAlloc {
                    dest: result_ptr,
                    count: 1,
                }),
            );

            for (block, value, _span) in branch_results {
                self.use_basic_block(block);

                let adjusted_val = if self.get_value_type(value) != result_type {
                    self.coerce_to_union(value, result_type)
                } else {
                    value
                };

                self.emit_store(result_ptr, adjusted_val);
                self.emit_jmp(merge_block_id);
            }

            self.seal_block(merge_block_id);
            self.use_basic_block(merge_block_id);

            self.emit_load(result_ptr)
        } else {
            for (block, _, _) in branch_results {
                self.use_basic_block(block);
                self.emit_jmp(merge_block_id);
            }

            self.seal_block(merge_block_id);
            self.use_basic_block(merge_block_id);
            self.emit_void()
        };

        self.check_expected(result, expr_span, expected_type)
    }
}
