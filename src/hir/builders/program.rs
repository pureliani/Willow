use std::collections::{HashMap, HashSet};

use crate::{
    ast::{
        decl::{Declaration, FnDecl},
        expr::ExprKind,
        stmt::StmtKind,
        ModulePath, Position, SymbolId,
    },
    compile::ParallelParseResult,
    globals::next_generic_declaration_id,
    hir::{
        builders::{
            Builder, BuilderContext, CheckedFunctionDecl, FunctionBodyKind,
            FunctionParam, GenericDeclaration, InGlobal, InModule, Module,
        },
        errors::SemanticError,
        types::checked_declaration::CheckedDeclaration,
        utils::scope::{Scope, ScopeKind},
    },
};

impl<'a, C: BuilderContext> Builder<'a, C> {
    /// Creates an isolated dummy builder which guarantees that
    /// all declarations created by it are cleaned up afterwards.
    /// returns any semantic errors caught during the dummy build
    pub fn with_dummy_builder<NewC: BuilderContext, F>(
        &mut self,
        context: NewC,
        scope: Scope,
        f: F,
    ) -> Vec<SemanticError>
    where
        F: FnOnce(&mut Builder<'_, NewC>),
    {
        let mut temp_errors = Vec::new();
        let mut temp_own_declarations = HashSet::new();

        let mut dummy_builder = Builder {
            context,
            program: self.program,
            errors: &mut temp_errors,
            current_scope: scope,
            own_declarations: &mut temp_own_declarations,
        };

        f(&mut dummy_builder);

        for id in &temp_own_declarations {
            self.program.declarations.remove(id);
        }

        self.program
            .monomorphizations
            .retain(|_, v| !temp_own_declarations.contains(v));

        temp_errors
    }
}

impl<'a> Builder<'a, InGlobal> {
    pub fn build(&mut self, mut modules: Vec<ParallelParseResult>) {
        for m in &modules {
            let module_scope = self
                .current_scope
                .enter(ScopeKind::File, Position::default());

            self.program.modules.insert(
                m.path.clone(),
                Module {
                    path: m.path.clone(),
                    root_scope: module_scope,
                },
            );
        }

        let empty_subs = HashMap::new();

        for m in &modules {
            let mut module_builder = self.as_module(m.path.clone());

            for decl in &m.declarations {
                match decl {
                    Declaration::TypeAlias(alias) => {
                        module_builder.build_type_alias_decl(
                            alias.clone(),
                            alias.identifier.span.clone(),
                            &empty_subs,
                        );
                    }
                    Declaration::Fn(f) => {
                        module_builder.register_fn_signature(f);
                    }
                }
            }
        }

        for m in std::mem::take(&mut modules) {
            let mut module_builder = self.as_module(m.path.clone());

            for stmt in m.statements {
                match stmt.kind {
                    StmtKind::From { path, items } => {
                        module_builder.build_from_stmt(path, items, stmt.span);
                    }
                    StmtKind::TypeAliasDecl(decl) => {
                        if !decl.generic_params.is_empty() {
                            module_builder.early_check_generic_type_alias(decl);
                        }
                    }
                    StmtKind::Expression(expr) => {
                        if let ExprKind::Fn(f) = expr.kind {
                            if f.generic_params.is_empty() {
                                if let Err(e) =
                                    module_builder.build_fn_body(*f, &empty_subs)
                                {
                                    module_builder.errors.push(e);
                                };
                            } else {
                                module_builder.early_check_generic_fn(*f);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn as_module(&mut self, path: ModulePath) -> Builder<'_, InModule> {
        let scope = self.program.modules.get(&path).unwrap().root_scope.clone();
        Builder {
            context: InModule { path },
            program: self.program,
            errors: self.errors,
            current_scope: scope,
            own_declarations: self.own_declarations,
        }
    }
}

impl<'a> Builder<'a, InModule> {
    fn register_fn_signature(&mut self, f: &FnDecl) {
        let decl_name = f.identifier.name;

        if !f.generic_params.is_empty() {
            let gen_id = next_generic_declaration_id();

            self.program.generic_declarations.insert(
                gen_id,
                GenericDeclaration::Function {
                    decl: f.clone(),
                    decl_scope: self.current_scope.clone(),
                    has_errors: false,
                },
            );

            self.current_scope
                .map_name_to_symbol(decl_name, SymbolId::Generic(gen_id));
            return;
        }

        let empty_subs = HashMap::new();
        let checked_params = self.check_params(&f.params, &empty_subs);
        let checked_return_type = self.check_type_annotation(&f.return_type, &empty_subs);

        let function_params = checked_params
            .into_iter()
            .map(|p| FunctionParam {
                identifier: p.identifier,
                ty: p.ty,
            })
            .collect();

        let decl_id = f.id;

        let function = CheckedFunctionDecl {
            id: decl_id,
            identifier: f.identifier.clone(),
            params: function_params,
            return_type: checked_return_type,
            is_exported: f.is_exported,
            body: FunctionBodyKind::NotBuilt,
        };

        self.program
            .declarations
            .insert(decl_id, CheckedDeclaration::Function(function));

        self.own_declarations.insert(decl_id);

        self.current_scope
            .map_name_to_symbol(decl_name, SymbolId::Concrete(decl_id));
    }
}
