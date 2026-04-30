use crate::{
    ast::{decl::Declaration, IdentifierNode},
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{InstrId, InstructionKind, MakeLiteralKind},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_identifier_expr(&mut self, identifier: IdentifierNode) -> InstrId {
        let span = identifier.span.clone();

        let decl_id = match self.current_scope.lookup(identifier.name) {
            Some(id) => id,
            None => {
                return self.report_error_and_get_poison(SemanticError {
                    span: span.clone(),
                    kind: SemanticErrorKind::UndeclaredIdentifier(identifier),
                });
            }
        };

        let decl = self.program.declarations.get(&decl_id).unwrap();

        match decl {
            Declaration::ExternFn(_) => self.push_instruction(
                InstructionKind::MakeLiteral(MakeLiteralKind::Fn(decl_id)),
                span,
            ),
            Declaration::Fn(_) => self.push_instruction(
                InstructionKind::MakeLiteral(MakeLiteralKind::Fn(decl_id)),
                span,
            ),
            Declaration::Var(_) | Declaration::Param(_) => {
                self.read_variable(self.context.block_id, decl_id)
            }
            Declaration::TypeAlias(_) | Declaration::GenericParameter(_) => self
                .report_error_and_get_poison(SemanticError {
                    span: span.clone(),
                    kind: SemanticErrorKind::CannotUseTypeDeclarationAsValue,
                }),
        }
    }
}
