#![allow(clippy::too_many_arguments)]

use std::collections::{BTreeSet, HashMap};

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        DeclarationId, GenericDeclarationId, IdentifierNode, Span, SymbolId,
    },
    compile::interner::{GenericSubstitutions, TypeId},
    hir::{
        builders::{Builder, FunctionBodyKind, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{
                CheckedDeclaration, CheckedParam, FnType, GenericDeclaration,
            },
            checked_type::{LiteralType, SpannedType, Type},
        },
        utils::{facts::narrowed_type::NarrowedTypeFact, place::Place, scope::ScopeKind},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_fn_call_expr(
        &mut self,
        left: Expr,
        args: Vec<Expr>,
        span: Span,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        if let ExprKind::Identifier(ident) = &left.kind {
            if let Some(SymbolId::Generic(gen_id)) = self.current_scope.lookup(ident.name)
            {
                return self.build_inferred_generic_call(
                    gen_id,
                    ident,
                    &args,
                    span,
                    expected_type,
                    substitutions,
                );
            }
        }

        let callee_decl_id = match &left.kind {
            ExprKind::Identifier(ident) => match self.current_scope.lookup(ident.name) {
                Some(SymbolId::Concrete(id)) => Some(id),
                _ => None,
            },
            _ => None,
        };

        let func_id = self.build_expr(left, None, substitutions);
        let func_type = self.get_value_type(func_id);

        let (params, return_type) = match self.types.resolve(func_type) {
            Type::Literal(LiteralType::Fn(decl_id)) => {
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
            Type::IndirectFn(FnType {
                params,
                return_type,
            }) => (params.clone(), return_type.clone()),
            Type::Literal(LiteralType::Unknown) => {
                return self.new_value_id(self.types.unknown())
            }
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

        let mut final_args = Vec::with_capacity(args.len());

        for (arg_expr, checked_param) in args.iter().zip(params) {
            let val_id =
                self.build_expr(arg_expr.clone(), Some(&checked_param.ty), substitutions);

            if self.get_value_type(val_id) == self.types.unknown() {
                return self.new_value_id(self.types.unknown());
            }

            final_args.push(val_id);
        }

        self.finalize_fn_call(
            func_id,
            return_type.id,
            final_args,
            callee_decl_id,
            &args,
            span,
            expected_type,
        )
    }

    fn build_inferred_generic_call(
        &mut self,
        gen_id: GenericDeclarationId,
        ident: &IdentifierNode,
        args: &[Expr],
        span: Span,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let generic_decl = self
            .program
            .generic_declarations
            .get(&gen_id)
            .unwrap()
            .clone();

        let (fn_decl, decl_scope) = match generic_decl {
            GenericDeclaration::Function {
                decl,
                decl_scope,
                has_errors: _,
            } => (decl, decl_scope),
            _ => {
                return self.report_error_and_get_poison(SemanticError {
                    span: ident.span.clone(),
                    kind: SemanticErrorKind::CannotCall(self.types.unknown()),
                });
            }
        };

        if args.len() != fn_decl.params.len() {
            return self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::FnArgumentCountMismatch {
                    expected: fn_decl.params.len(),
                    received: args.len(),
                },
                span: span.clone(),
            });
        }

        let mut evaluated_args = Vec::with_capacity(args.len());
        for arg_expr in args {
            let arg_span = arg_expr.span.clone();
            let val_id = self.build_expr(arg_expr.clone(), None, substitutions);
            let val_ty = self.get_value_type(val_id);
            evaluated_args.push((val_id, val_ty, arg_span));
        }

        let caller_scope = self.current_scope.clone();
        self.current_scope = decl_scope.enter(ScopeKind::GenericParams, span.start);
        for param in &fn_decl.generic_params {
            self.current_scope.map_name_to_symbol(
                param.identifier.name,
                SymbolId::GenericParameter(param.identifier.name),
            );
        }

        let mut inferred = HashMap::new();
        let mut inference_failed = false;
        for (param_decl, (_, actual_ty, _)) in
            fn_decl.params.iter().zip(evaluated_args.iter())
        {
            if let Err(e) = self.infer_type_arguments(
                &param_decl.constraint,
                *actual_ty,
                &mut inferred,
            ) {
                self.errors.push(e);
                inference_failed = true;
            }
        }

        self.current_scope = caller_scope;

        if inference_failed {
            return self.new_value_id(self.types.unknown());
        }

        let mut type_args = Vec::new();
        for gen_param in &fn_decl.generic_params {
            if let Some(&ty) = inferred.get(&gen_param.identifier.name) {
                type_args.push(ty);
            } else {
                return self.report_error_and_get_poison(SemanticError {
                    span: span.clone(),
                    kind: SemanticErrorKind::CannotInferGenericArgument(
                        gen_param.identifier.clone(),
                    ),
                });
            }
        }

        let concrete_func_id =
            match self.monomorphize_function(gen_id, type_args, span.clone()) {
                Ok(id) => id,
                Err(()) => return self.new_value_id(self.types.unknown()),
            };

        let concrete_decl = self.program.declarations.get(&concrete_func_id).unwrap();
        let (concrete_params, return_type_id) =
            if let CheckedDeclaration::Function(f) = concrete_decl {
                (f.params.clone(), f.return_type.id)
            } else {
                unreachable!()
            };

        let mut final_args = Vec::with_capacity(args.len());

        for ((val_id, _val_ty, arg_span), param) in
            evaluated_args.into_iter().zip(concrete_params)
        {
            let adjusted_val = self.check_expected(val_id, arg_span, Some(&param.ty));
            final_args.push(adjusted_val);
        }

        let func_val = self.emit_const_fn(concrete_func_id);

        self.finalize_fn_call(
            func_val,
            return_type_id,
            final_args,
            Some(concrete_func_id),
            args,
            span,
            expected_type,
        )
    }

    fn finalize_fn_call(
        &mut self,
        func_val: ValueId,
        return_type: TypeId,
        final_args: Vec<ValueId>,
        callee_decl_id: Option<DeclarationId>,
        arg_exprs: &[Expr],
        span: Span,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let result = self.emit_call(func_val, final_args, return_type);

        if let Some(decl_id) = callee_decl_id {
            self.apply_callee_effects(decl_id, arg_exprs);
        }

        if return_type == self.types.never() {
            self.emit_unreachable();
            return self.new_value_id(self.types.unknown());
        }

        self.check_expected(result, span, expected_type)
    }

    fn apply_callee_effects(
        &mut self,
        callee_decl_id: DeclarationId,
        arg_exprs: &[Expr],
    ) {
        let effects = match self.program.declarations.get(&callee_decl_id) {
            Some(CheckedDeclaration::Function(f)) => match &f.body {
                FunctionBodyKind::Internal(cfg) => cfg.effects.clone(),
                FunctionBodyKind::External => return,
                FunctionBodyKind::NotBuilt => return,
            },
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
