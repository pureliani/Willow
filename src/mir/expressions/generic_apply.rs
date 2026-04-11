use crate::{
    ast::{
        expr::{Expr, ExprKind},
        type_annotation::TypeAnnotation,
        Span, SymbolId,
    },
    compile::interner::GenericSubstitutions,
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{checked_declaration::GenericDeclaration, checked_type::SpannedType},
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

                if let GenericDeclaration::Function { decl: fn_decl, .. } = generic_decl {
                    if fn_decl.generic_params.len() != type_args.len() {
                        return self.report_error_and_get_poison(SemanticError {
                            span,
                            kind: SemanticErrorKind::GenericArgumentCountMismatch {
                                expected: fn_decl.generic_params.len(),
                                received: type_args.len(),
                            },
                        });
                    }

                    let mut checked_type_args = Vec::new();
                    for arg in type_args {
                        checked_type_args
                            .push(self.check_type_annotation(&arg, substitutions).id);
                    }

                    let concrete_id = self.monomorphize_function(
                        gen_id,
                        checked_type_args,
                        span.clone(),
                    );

                    let result = self.emit_const_fn(concrete_id);
                    self.check_expected(result, span, expected_type)
                } else {
                    self.report_error_and_get_poison(SemanticError {
                        span,
                        kind: SemanticErrorKind::CannotUseTypeDeclarationAsValue,
                    })
                }
            }
            SymbolId::Concrete(_) | SymbolId::GenericParameter(_) => self
                .report_error_and_get_poison(SemanticError {
                    span,
                    kind: SemanticErrorKind::CannotApplyTypeArguments,
                }),
        }
    }
}
