use std::collections::HashMap;

use crate::{
    ast::{
        decl::{Declaration, FnDecl},
        expr::ExprKind,
        stmt::StmtKind,
        ModulePath, Position, SymbolId,
    },
    compile::ParallelParseResult,
    globals::next_generic_declaration_id,
    mir::{
        builders::{
            Builder, CheckedFunctionDecl, FunctionBodyKind, FunctionParam,
            GenericDeclaration, InGlobal, InModule, Module,
        },
        types::checked_declaration::CheckedDeclaration,
        utils::scope::ScopeKind,
    },
};

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
                    StmtKind::Expression(expr) => {
                        if let ExprKind::Fn(f) = expr.kind {
                            if f.generic_params.is_empty() {
                                if let Err(e) =
                                    module_builder.build_fn_body(*f, &empty_subs)
                                {
                                    module_builder.errors.push(e);
                                };
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
            condition_facts: self.condition_facts,
            current_facts: self.current_facts,
            incomplete_fact_merges: self.incomplete_fact_merges,
            aliases: self.aliases,
            types: self.types,
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
                decl_id: None,
                value_id: None,
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

        self.current_scope
            .map_name_to_symbol(decl_name, SymbolId::Concrete(decl_id));
    }
}
