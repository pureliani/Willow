use crate::{
    ast::{IdentifierNode, SymbolId},
    compile::interner::GenericSubstitutions,
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, GenericDeclaration},
            checked_type::SpannedType,
        },
        utils::place::Place,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_identifier_expr(
        &mut self,
        identifier: IdentifierNode,
        expected_type: Option<&SpannedType>,
        _substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let span = identifier.span.clone();

        let symbol_id = match self.current_scope.lookup(identifier.name) {
            Some(id) => id,
            None => {
                return self.report_error_and_get_poison(SemanticError {
                    span: identifier.span.clone(),
                    kind: SemanticErrorKind::UndeclaredIdentifier(identifier),
                });
            }
        };

        match symbol_id {
            SymbolId::Concrete(decl_id) => {
                let decl = self.program.declarations.get(&decl_id).unwrap();

                let result = match decl {
                    CheckedDeclaration::Function(_) => self.emit_const_fn(decl_id),
                    CheckedDeclaration::Var(_) => {
                        let place = self
                            .aliases
                            .get(&decl_id)
                            .cloned()
                            .unwrap_or(Place::Var(decl_id));

                        self.read_place(&place)
                    }
                    CheckedDeclaration::TypeAlias(_) => {
                        self.report_error_and_get_poison(SemanticError {
                            span: identifier.span.clone(),
                            kind: SemanticErrorKind::CannotUseTypeDeclarationAsValue,
                        })
                    }
                };

                self.check_expected(result, span, expected_type)
            }
            SymbolId::Generic(gen_id) => {
                let generic_decl =
                    self.program.generic_declarations.get(&gen_id).unwrap();

                let error_kind = match generic_decl {
                    GenericDeclaration::Function { .. } => {
                        SemanticErrorKind::MissingGenericArguments
                    }
                    GenericDeclaration::TypeAlias { .. } => {
                        SemanticErrorKind::CannotUseTypeDeclarationAsValue
                    }
                };

                let result = self.report_error_and_get_poison(SemanticError {
                    span: identifier.span.clone(),
                    kind: error_kind,
                });

                self.check_expected(result, span, expected_type)
            }
        }
    }
}
