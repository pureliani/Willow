use crate::{
    ast::IdentifierNode,
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{checked_declaration::CheckedDeclaration, checked_type::SpannedType},
        utils::place::Place,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_identifier_expr(
        &mut self,
        identifier: IdentifierNode,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let span = identifier.span.clone();
        let decl_id = match self.current_scope.lookup(identifier.name) {
            Some(id) => id,
            None => {
                return self.report_error_and_get_poison(SemanticError {
                    span: identifier.span.clone(),
                    kind: SemanticErrorKind::UndeclaredIdentifier(identifier),
                });
            }
        };

        let decl = self.program.declarations.get(&decl_id).unwrap();

        let result = match decl {
            CheckedDeclaration::Function(_) => self.emit_const_fn(decl_id),
            CheckedDeclaration::Var(_) => {
                let place = self
                    .aliases
                    .get(&decl_id)
                    .cloned()
                    .unwrap_or(Place::Var(decl_id));

                self.read_place(&place, identifier.span)
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
}
