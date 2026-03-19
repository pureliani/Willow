use crate::{
    ast::{
        expr::{Expr, ExprKind},
        DeclarationId, Span,
    },
    compile::interner::TypeId,
    mir::{
        builders::{Builder, ExpectBody, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{Instruction, ReinterpretInstr},
        types::{
            checked_declaration::CheckedDeclaration,
            checked_type::{SpannedType, Type},
        },
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
        let func_type = self.get_value_type(func_id).clone();

        let (params, return_type) = match func_type {
            Type::Fn(f) => (f.params, *f.return_type),
            Type::Unknown => return self.new_value_id(Type::Unknown),
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

            if val_ty == &Type::Unknown {
                return self.new_value_id(Type::Unknown);
            }

            evaluated_args.push((val_id, arg_span));
        }

        if let Err(e) = self.check_argument_aliasing(&evaluated_args) {
            return self.report_error_and_get_poison(e);
        }

        let result = self.emit_call(
            func_id,
            evaluated_args.iter().map(|a| a.0).collect(),
            return_type.kind,
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
        evaluated: &[(ValueId, Span)],
    ) {
        let effects = match self.program.declarations.get(&callee_decl_id) {
            Some(CheckedDeclaration::Function(f)) => f.expect_body().effects.clone(),
            _ => return,
        };

        for mutation in &effects.mutations {
            let arg_expr = &arg_exprs[mutation.param_index];
            let arg_span = &evaluated[mutation.param_index].1;

            if let Some((decl_id, Some(new_type), _)) = self.resolve_narrow_target(
                arg_expr,
                Some(mutation.exit_type.clone()),
                None,
            ) {
                self.apply_effect_mutation(decl_id, new_type, arg_span.clone());
            }
        }
    }

    fn apply_effect_mutation(
        &mut self,
        decl_id: DeclarationId,
        new_type: TypeId,
        span: Span,
    ) {
        let current_val =
            self.read_variable(decl_id, self.context.block_id, span.clone());
        let current_ty = self.get_value_type(current_val).clone();

        if current_ty == new_type {
            return;
        }

        let new_val = self.new_value_id(new_type);
        self.push_instruction(Instruction::Reinterpret(ReinterpretInstr {
            src: current_val,
            dest: new_val,
        }));

        self.write_variable(decl_id, self.context.block_id, new_val);
    }
}
