use crate::{
    ast::{
        decl::{Declaration, TypeAliasDecl},
        Span,
    },
    hir::{
        builders::{Builder, InModule},
        errors::{SemanticError, SemanticErrorKind},
    },
};

impl<'a> Builder<'a, InModule> {
    pub fn build_type_alias_decl(&mut self, decl: TypeAliasDecl, span: Span) {
        if !self.current_scope.is_file_scope() {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::TypeAliasMustBeDeclaredAtTopLevel,
                span,
            });
            return;
        }

        let decl_id = decl.id;
        let decl_name = decl.identifier.name;
        self.program
            .declarations
            .insert(decl_id, Declaration::TypeAlias(decl));
        self.current_scope.map_name_to_symbol(decl_name, decl_id);
    }
}
