use std::collections::HashMap;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        type_annotation::TypeAnnotation,
        Span, SymbolId,
    },
    compile::interner::GenericSubstitutions,
    globals::next_declaration_id,
    mir::{
        builders::{
            Builder, CheckedFunctionDecl, FunctionBodyKind, FunctionParam, InBlock,
            ValueId,
        },
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, GenericDeclaration},
            checked_type::SpannedType,
        },
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_generic_apply_expr(
        &mut self,
        left: Expr,
        type_args: Vec<TypeAnnotation>,
        span: Span,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let ident = match left.kind {
            ExprKind::Identifier(id) => id,
            _ => {
                return self.report_error_and_get_poison(SemanticError {
                    span: left.span,
                    kind: SemanticErrorKind::CannotApplyTypeArguments,
                });
            }
        };

        let symbol_id = match self.current_scope.lookup(ident.name) {
            Some(id) => id,
            None => {
                return self.report_error_and_get_poison(SemanticError {
                    span: ident.span.clone(),
                    kind: SemanticErrorKind::UndeclaredIdentifier(ident),
                });
            }
        };

        match symbol_id {
            SymbolId::Generic(gen_id) => {
                let generic_decl = self
                    .program
                    .generic_declarations
                    .get(&gen_id)
                    .unwrap()
                    .clone();

                if let GenericDeclaration::Function {
                    decl: fn_decl,
                    decl_scope,
                } = generic_decl
                {
                    if fn_decl.generic_params.len() != type_args.len() {
                        return self.report_error_and_get_poison(SemanticError {
                            span,
                            kind: SemanticErrorKind::GenericArgumentCountMismatch {
                                expected: fn_decl.generic_params.len(),
                                received: type_args.len(),
                            },
                        });
                    }

                    let mut evaluated_args = Vec::new();
                    for arg in type_args {
                        evaluated_args
                            .push(self.check_type_annotation(&arg, substitutions).id);
                    }

                    let cache_key = (gen_id, evaluated_args.clone());
                    if let Some(&concrete_id) =
                        self.program.monomorphizations.get(&cache_key)
                    {
                        let result = self.emit_const_fn(concrete_id);
                        return self.check_expected(result, span, expected_type);
                    }

                    let mut inner_substitutions = HashMap::new();
                    for (param, arg_ty) in
                        fn_decl.generic_params.iter().zip(evaluated_args)
                    {
                        inner_substitutions.insert(param.identifier.name, arg_ty);
                    }

                    let new_decl_id = next_declaration_id();
                    self.program
                        .monomorphizations
                        .insert(cache_key, new_decl_id);

                    let caller_scope = self.current_scope.clone();
                    self.current_scope = decl_scope.clone();

                    let checked_params =
                        self.check_params(&fn_decl.params, &inner_substitutions);
                    let checked_return_type = self.check_type_annotation(
                        &fn_decl.return_type,
                        &inner_substitutions,
                    );

                    let function_params = checked_params
                        .into_iter()
                        .map(|p| FunctionParam {
                            identifier: p.identifier,
                            ty: p.ty,
                            decl_id: None,
                            value_id: None,
                        })
                        .collect();

                    let concrete_func = CheckedFunctionDecl {
                        id: new_decl_id,
                        identifier: fn_decl.identifier.clone(),
                        params: function_params,
                        return_type: checked_return_type,
                        is_exported: false,
                        body: FunctionBodyKind::NotBuilt,
                    };

                    self.program
                        .declarations
                        .insert(new_decl_id, CheckedDeclaration::Function(concrete_func));

                    let mut concrete_ast = fn_decl.clone();
                    concrete_ast.id = new_decl_id;

                    if let Err(e) = self
                        .as_module()
                        .build_fn_body(concrete_ast, &inner_substitutions)
                    {
                        self.errors.push(e);
                    }

                    self.current_scope = caller_scope;

                    let result = self.emit_const_fn(new_decl_id);
                    self.check_expected(result, span, expected_type)
                } else {
                    self.report_error_and_get_poison(SemanticError {
                        span,
                        kind: SemanticErrorKind::CannotUseTypeDeclarationAsValue,
                    })
                }
            }
            SymbolId::Concrete(_) => self.report_error_and_get_poison(SemanticError {
                span,
                kind: SemanticErrorKind::CannotApplyTypeArguments,
            }),
        }
    }
}
