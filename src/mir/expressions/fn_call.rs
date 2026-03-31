use std::collections::BTreeSet;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        DeclarationId, Span,
    },
    compile::interner::TypeId,
    mir::{
        builders::{Builder, ExpectBody, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, CheckedParam, FnType},
            checked_type::{SpannedType, Type},
        },
        utils::{facts::narrowed_type::NarrowedTypeFact, place::Place},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_fn_call_expr(
        &mut self,
        left: Expr,
        args: Vec<Expr>,
        span: Span,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let callee_decl_id = match &left.kind {
            ExprKind::Identifier(ident) => self.current_scope.lookup(ident.name),
            _ => None,
        };

        let func_id = self.build_expr(left, None);
        let func_type = self.get_value_type(func_id);

        let (params, return_type) = match self.types.resolve(func_type) {
            Type::Fn(FnType::Direct(decl_id)) => {
                if let Some(CheckedDeclaration::Function(f)) =
                    self.program.declarations.get(&decl_id)
                {
                    let checked_params = f
                        .params
                        .iter()
                        .map(|p| CheckedParam {
                            identifier: p.identifier.clone(),
                            ty: p.ty.clone(),
                        })
                        .collect();

                    (checked_params, f.return_type.clone())
                } else {
                    panic!("INTERNAL COMPILER ERROR: Expected function declaration");
                }
            }
            Type::Fn(FnType::Indirect {
                params,
                return_type,
            }) => (params.clone(), return_type.clone()),
            Type::Unknown => return self.new_value_id(self.types.unknown()),
            _ => {
                return self.report_error_and_get_poison(SemanticError {
                    kind: SemanticErrorKind::CannotCall(func_type),
                    span: span.clone(),
                });
            }
        };

        if args.len() != params.len() {
            return self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::FnArgumentCountMismatch {
                    expected: params.len(),
                    received: args.len(),
                },
                span: span.clone(),
            });
        }

        let mut evaluated_args = Vec::with_capacity(args.len());

        for (arg_expr, checked_param) in args.iter().zip(params) {
            let arg_span = arg_expr.span.clone();

            let val_id = self.build_expr(arg_expr.clone(), Some(&checked_param.ty));
            let val_ty = self.get_value_type(val_id);

            if val_ty == self.types.unknown() {
                return self.new_value_id(self.types.unknown());
            }

            evaluated_args.push((val_id, arg_span));
        }

        if let Err(e) = self.check_argument_aliasing(&evaluated_args) {
            return self.report_error_and_get_poison(e);
        }

        let result = self.emit_call(
            func_id,
            evaluated_args.iter().map(|a| a.0).collect(),
            return_type.id,
        );

        if let Some(decl_id) = callee_decl_id {
            self.apply_callee_effects(decl_id, &args, &evaluated_args);
        }

        self.check_expected(result, span, expected_type)
    }

    fn check_argument_aliasing(
        &self,
        args: &[(ValueId, Span)],
    ) -> Result<(), SemanticError> {
        let val_ids: Vec<ValueId> = args.iter().map(|(v, _)| *v).collect();

        if let Some(conflict) = self.ptg.check_aliasing(&val_ids) {
            return Err(SemanticError {
                kind: SemanticErrorKind::ArgumentAliasing {
                    passed_arg_span: args[conflict.arg_i].1.clone(),
                    passed_path: conflict.path_i,
                    aliased_arg_span: args[conflict.arg_j].1.clone(),
                    aliased_path: conflict.path_j,
                },
                span: args[conflict.arg_i].1.clone(),
            });
        }

        Ok(())
    }

    fn apply_callee_effects(
        &mut self,
        callee_decl_id: DeclarationId,
        arg_exprs: &[Expr],
        _evaluated: &[(ValueId, Span)],
    ) {
        let effects = match self.program.declarations.get(&callee_decl_id) {
            Some(CheckedDeclaration::Function(f)) => f.expect_body().effects.clone(),
            _ => return,
        };

        for mutation in &effects.mutations {
            let arg_expr = &arg_exprs[mutation.param_index];
            let exit_type_id = mutation.exit_type;

            if let Some(place) = self.resolve_narrow_target(arg_expr) {
                self.apply_effect_mutation(place, exit_type_id);
            }
        }
    }

    fn apply_effect_mutation(&mut self, place: Place, new_type_id: TypeId) {
        let mut existing_facts = self.read_fact_from_block(self.context.block_id, &place);

        if let Some(variants) = self.types.get_union_variants(new_type_id) {
            existing_facts.insert(NarrowedTypeFact { variants });
        } else {
            existing_facts.insert(NarrowedTypeFact {
                variants: BTreeSet::from([new_type_id]),
            });
        }

        self.write_fact(self.context.block_id, &place, existing_facts);
    }
}
