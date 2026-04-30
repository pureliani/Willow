use crate::{
    ast::{decl::Declaration, expr::ExprKind, stmt::StmtKind, ModulePath, Position},
    compile::ParallelParseResult,
    hir::{
        builders::{Builder, InGlobal, InModule, Module},
        errors::{SemanticError, SemanticErrorKind},
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
                    _ => {}
                }
            }

            for stmt in &m.statements {
                match &stmt.kind {
                    StmtKind::From { path, items } => {
                        module_builder.build_from_stmt(
                            path.clone(),
                            items.clone(),
                            stmt.span.clone(),
                        );
                    }
                    StmtKind::VarDecl(var_decl) => {
                        module_builder.errors.push(SemanticError {
                            kind: SemanticErrorKind::CannotDeclareGlobalVariable,
                            span: var_decl.identifier.span.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }

        for m in std::mem::take(&mut modules) {
            let mut module_builder = self.as_module(m.path.clone());

            for stmt in m.statements {
                match stmt.kind {
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
            current_def: self.current_def,
            incomplete_phis: self.incomplete_phis,
            current_memory_def: self.current_memory_def,
            incomplete_memory_phis: self.incomplete_memory_phis,
        }
    }
}

impl<'a> Builder<'a, InModule> {
    fn register_fn_signature(&mut self, f: &crate::ast::decl::FnDecl) {
        let decl_id = f.id;
        let decl_name = f.identifier.name;

        self.program
            .declarations
            .insert(decl_id, Declaration::Fn(f.clone()));

        self.current_scope.map_name_to_symbol(decl_name, decl_id);
    }
}
