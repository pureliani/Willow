use crate::{
    ast::{decl::TypeAliasDecl, Span, SymbolId},
    compile::interner::GenericSubstitutions,
    globals::next_generic_declaration_id,
    hir::{
        builders::{Builder, InModule},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_declaration::{
            CheckedDeclaration, CheckedTypeAliasDecl, GenericDeclaration,
        },
    },
};

impl<'a> Builder<'a, InModule> {
    pub fn build_type_alias_decl(
        &mut self,
        type_alias_decl: TypeAliasDecl,
        span: Span,
        substitutions: &GenericSubstitutions,
    ) {
        if !self.current_scope.is_file_scope() {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::TypeAliasMustBeDeclaredAtTopLevel,
                span,
            });
            return;
        }

        let decl_name = type_alias_decl.identifier.name;

        if !type_alias_decl.generic_params.is_empty() {
            let gen_id = next_generic_declaration_id();

            self.program.generic_declarations.insert(
                gen_id,
                GenericDeclaration::TypeAlias {
                    decl: type_alias_decl,
                    decl_scope: self.current_scope.clone(),
                    has_errors: false,
                },
            );

            self.current_scope
                .map_name_to_symbol(decl_name, SymbolId::Generic(gen_id));
            return;
        }

        let resolved_type =
            self.check_type_annotation(&type_alias_decl.value, substitutions);

        let decl_id = type_alias_decl.id;

        let checked_type_alias_decl = CheckedTypeAliasDecl {
            id: decl_id,
            documentation: type_alias_decl.documentation,
            identifier: type_alias_decl.identifier.clone(),
            span,
            value: Box::new(resolved_type),
            is_exported: type_alias_decl.is_exported,
        };

        self.program.declarations.insert(
            decl_id,
            CheckedDeclaration::TypeAlias(checked_type_alias_decl),
        );

        self.own_declarations.insert(decl_id);

        self.current_scope
            .map_name_to_symbol(decl_name, SymbolId::Concrete(decl_id));
    }
}
