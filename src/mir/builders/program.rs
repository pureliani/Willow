use crate::{
    ast::{
        decl::{Declaration, FnDecl},
        expr::ExprKind,
        stmt::StmtKind,
        ModulePath, Position,
    },
    compile::ParallelParseResult,
    mir::{
        builders::{
            Builder, Function, FunctionBodyKind, FunctionParam, InGlobal, InModule,
            Module,
        },
        types::checked_declaration::CheckedDeclaration,
        utils::{
            check_type::{check_params, check_type_annotation, TypeCheckerContext},
            scope::ScopeKind,
        },
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

        for m in &modules {
            let mut module_builder = self.as_module(m.path.clone());

            for decl in &m.declarations {
                match decl {
                    Declaration::TypeAlias(alias) => {
                        module_builder.build_type_alias_decl(
                            alias.clone(),
                            alias.identifier.span.clone(),
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
                    StmtKind::From { path, identifiers } => {
                        module_builder.build_from_stmt(path, identifiers, stmt.span);
                    }
                    StmtKind::Expression(expr) => {
                        if let ExprKind::Fn(f) = expr.kind {
                            if let Err(e) = module_builder.build_fn_body(*f) {
                                module_builder.errors.push(e);
                            };
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
            current_defs: self.current_defs,
            incomplete_phis: self.incomplete_phis,
            type_predicates: self.type_predicates,
            ptg: self.ptg,
            aliases: self.aliases,
        }
    }
}

impl<'a> Builder<'a, InModule> {
    fn register_fn_signature(&mut self, f: &FnDecl) {
        let mut type_ctx = TypeCheckerContext {
            scope: self.current_scope.clone(),
            declarations: &self.program.declarations,
            errors: self.errors,
        };

        let checked_params = check_params(&mut type_ctx, &f.params);
        let checked_return_type = check_type_annotation(&mut type_ctx, &f.return_type);

        let function_params = checked_params
            .into_iter()
            .map(|p| FunctionParam {
                identifier: p.identifier,
                ty: p.ty,
                decl_id: None,
                value_id: None,
            })
            .collect();

        let function = Function {
            id: f.id,
            identifier: f.identifier.clone(),
            params: function_params,
            return_type: checked_return_type,
            is_exported: f.is_exported,
            body: FunctionBodyKind::NotBuilt,
        };

        self.program
            .declarations
            .insert(f.id, CheckedDeclaration::Function(function));
        self.current_scope.map_name_to_decl(f.identifier.name, f.id);
    }
}
