use std::collections::HashSet;

use crate::{
    ast::{
        expr::{BlockContents, Expr},
        Span,
    },
    compile::interner::TypeId,
    mir::{
        builders::{BasicBlockId, Builder, InBlock, PhiSource, TypePredicate, ValueId},
        errors::{SemanticError, SemanticErrorKind},
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
                span: condition_span.clone(),
                kind: Type::Bool,
            });

            let cond_id = self.build_expr(*condition, expected_condition_type.as_ref());
            let cond_ty = self.get_value_type(cond_id);

            if cond_ty == &Type::Unknown {
                return self.new_value_id(Type::Unknown);
            }

            let then_block_id = self.as_fn().new_bb();
            let next_cond_block_id = self.as_fn().new_bb();

            self.emit_cond_jmp(cond_id, then_block_id, next_cond_block_id);

            self.seal_block(then_block_id);
            self.use_basic_block(then_block_id);

            if let Some(preds) = self.type_predicates.get(&cond_id).cloned() {
                self.apply_predicate_list(&preds, true, &condition_span);
            }

            let (then_val, then_val_span) =
                self.build_codeblock_expr(body, expected_type, false);

            if self.bb().terminator.is_none() {
                branch_results.push((self.context.block_id, then_val, then_val_span));
                self.emit_jmp(merge_block_id);
            }

            self.use_basic_block(next_cond_block_id);

            if let Some(preds) = self.type_predicates.get(&cond_id).cloned() {
                self.apply_predicate_list(&preds, false, &condition_span);
            }

            current_cond_block_id = next_cond_block_id;
        }

        self.use_basic_block(current_cond_block_id);

        if let Some(else_body) = else_branch {
            let (else_val, else_val_span) =
                self.build_codeblock_expr(else_body, expected_type, false);

            if self.bb().terminator.is_none() {
                branch_results.push((self.context.block_id, else_val, else_val_span));
                self.emit_jmp(merge_block_id);
            }
        } else {
            self.emit_jmp(merge_block_id);
        }

        self.seal_block(current_cond_block_id);

        self.seal_block(merge_block_id);
        self.use_basic_block(merge_block_id);

        let result = if context == IfContext::Expression {
            if branch_results.is_empty() {
                return self.new_value_id(Type::Never);
            }

            let type_entries: Vec<Type> = branch_results
                .iter()
                .map(|(_, val, _)| self.get_value_type(*val).clone())
                .collect();

            let result_type = Type::make_union(type_entries);

            let phi_id = self.new_value_id(result_type.clone());
            let mut phi_sources: HashSet<PhiSource> = HashSet::new();

            for (block, value, span) in branch_results {
                let adjusted_val =
                    self.adjust_phi_source_value(block, value, result_type.clone(), span);
                phi_sources.insert(PhiSource {
                    from: block,
                    value: adjusted_val,
                });
            }

            self.insert_phi(merge_block_id, phi_id, phi_sources);
            phi_id
        } else {
            self.emit_void()
        };

        self.check_expected(result, expr_span, expected_type)
    }

    pub fn apply_type_predicate(
        &mut self,
        pred: &TypePredicate,
        new_type: TypeId,
        span: Span,
    ) {
        let current_val = self.read_variable(pred.decl_id, self.context.block_id, span);
        let current_ty = self.get_value_type(current_val).clone();

        if current_ty == new_type {
            return;
        }

        let narrowed_val = if current_ty.get_narrowed_variants().is_some() {
            if new_type.get_narrowed_variants().is_some() {
                self.emit_narrow_union(current_val, &new_type)
            } else {
                self.emit_unwrap_from_union(current_val, &new_type)
            }
        } else {
            // Type predicates cannot be applied to non union types
            return;
        };

        self.write_variable(pred.decl_id, self.context.block_id, narrowed_val);
    }

    pub fn apply_predicate_list(
        &mut self,
        preds: &[TypePredicate],
        use_true_side: bool,
        span: &Span,
    ) {
        for pred in preds {
            let ty = if use_true_side {
                &pred.on_true_type
            } else {
                &pred.on_false_type
            };

            if let Some(ty) = ty {
                self.apply_type_predicate(pred, ty.clone(), span.clone());
            }
        }
    }
}
