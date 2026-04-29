use std::collections::HashMap;

use crate::{
    ast::{decl::TypeAliasDecl, Span, SymbolId},
    compile::interner::GenericSubstitutions,
    globals::next_generic_declaration_id,
    hir::{
        builders::{Builder, InModule},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{
                CheckedDeclaration, CheckedTypeAliasDecl, GenericDeclaration,
            },
            checked_type::Type,
        },
        utils::scope::ScopeKind,
    },
};

impl<'a> Builder<'a, InModule> {
    pub fn early_check_generic_type_alias(&mut self, decl: TypeAliasDecl) {
        let mut generic_subs = HashMap::new();

        for param in &decl.generic_params {
            let constraint_ty = param.extends.as_ref().map(|constraint_ast| {
                self.check_type_annotation(constraint_ast, &generic_subs).id
            });

            let gen_ty = self.types.intern(&Type::GenericParam {
                identifier: param.identifier.clone(),
                extends: constraint_ty,
            });

            generic_subs.insert(param.identifier.name, gen_ty);
        }

        let check_scope = self
            .current_scope
            .enter(ScopeKind::GenericParams, decl.identifier.span.start);
        for param in &decl.generic_params {
            check_scope.map_name_to_symbol(
                param.identifier.name,
                SymbolId::GenericParameter(param.identifier.name),
            );
        }

        let context = InModule {
            path: self.context.path.clone(),
        };

        let temp_errors = self.with_dummy_builder(context, check_scope, |temp_builder| {
            temp_builder.check_type_annotation(&decl.value, &generic_subs);
        });

        if !temp_errors.is_empty() {
            self.errors.extend(temp_errors);

            if let Some(SymbolId::Generic(gen_id)) =
                self.current_scope.lookup(decl.identifier.name)
            {
                if let Some(GenericDeclaration::TypeAlias { has_errors, .. }) =
                    self.program.generic_declarations.get_mut(&gen_id)
                {
                    *has_errors = true;
                }
            }
        }
    }

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
