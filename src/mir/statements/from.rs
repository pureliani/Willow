use crate::{
    ast::{decl::FnDecl, stmt::ImportItem, ModulePath, Span, StringNode},
    mir::{
        builders::{
            Builder, CheckedFunctionDecl, FunctionBodyKind, FunctionParam, InModule,
        },
        errors::{SemanticError, SemanticErrorKind},
        types::checked_declaration::CheckedDeclaration,
    },
};
use std::path::PathBuf;
use std::sync::Arc;

pub fn is_linkable_external_file(extension: Option<&str>) -> bool {
    matches!(extension, Some("c") | Some("o") | Some("a"))
}

impl<'a> Builder<'a, InModule> {
    pub fn build_from_stmt(
        &mut self,
        path: StringNode,
        items: Vec<ImportItem>,
        span: Span,
    ) {
        if !self.current_scope.is_file_scope() {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::FromStatementMustBeDeclaredAtTopLevel,
                span,
            });
            return;
        }

        let path_str = &path.value;

        if path_str == "std" || path_str.starts_with("std/") {
            self.build_std_import(path_str, items, span);
            return;
        }

        let mut target_path_buf = PathBuf::from(&*self.context.path.0);
        target_path_buf.pop();
        target_path_buf.push(&path.value);

        let canonical_path = match target_path_buf.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                self.errors.push(SemanticError {
                    kind: SemanticErrorKind::ModuleNotFound(ModulePath(Arc::new(
                        target_path_buf,
                    ))),
                    span: path.span,
                });
                return;
            }
        };

        let ext = canonical_path.extension().and_then(|e| e.to_str());
        if is_linkable_external_file(ext) {
            self.program.foreign_links.insert(canonical_path.clone());

            for item in items {
                match item {
                    ImportItem::ExternFn(f) => self.register_extern_fn(f),
                    ImportItem::Symbol {
                        identifier,
                        alias: _,
                    } => {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::UndeclaredIdentifier(
                                identifier.clone(),
                            ),
                            span: identifier.span,
                        });
                    }
                }
            }
        } else {
            let module_path = ModulePath(Arc::new(canonical_path.clone()));
            let target_module = match self.program.modules.get(&module_path) {
                Some(m) => m,
                None => {
                    self.errors.push(SemanticError {
                        kind: SemanticErrorKind::ModuleNotFound(module_path),
                        span: path.span,
                    });
                    return;
                }
            };

            for item in items {
                match item {
                    ImportItem::Symbol {
                        identifier: imported_ident,
                        alias,
                    } => {
                        let not_exported_err = SemanticError {
                            span: imported_ident.span.clone(),
                            kind: SemanticErrorKind::SymbolNotExported {
                                module_path: module_path.clone(),
                                symbol: imported_ident.clone(),
                            },
                        };

                        if let Some(decl_id) =
                            target_module.root_scope.lookup(imported_ident.name)
                        {
                            let is_exported = match self
                                .program
                                .declarations
                                .get(&decl_id)
                            {
                                Some(CheckedDeclaration::Function(f)) => f.is_exported,
                                Some(CheckedDeclaration::TypeAlias(t)) => t.is_exported,
                                _ => false,
                            };

                            if is_exported {
                                let name_node = alias.unwrap_or(imported_ident);
                                self.current_scope
                                    .map_name_to_decl(name_node.name, decl_id);
                            } else {
                                self.errors.push(not_exported_err);
                            }
                        } else {
                            self.errors.push(not_exported_err);
                        }
                    }
                    ImportItem::ExternFn(f) => {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::UndeclaredIdentifier(
                                f.identifier.clone(),
                            ),
                            span: f.identifier.span,
                        });
                    }
                }
            }
        }
    }

    fn register_extern_fn(&mut self, f: FnDecl) {
        let checked_params = self.check_params(&f.params);
        let checked_return_type = self.check_type_annotation(&f.return_type);

        let function_params = checked_params
            .into_iter()
            .map(|p| FunctionParam {
                identifier: p.identifier,
                ty: p.ty,
                decl_id: None,
                value_id: None,
            })
            .collect();

        let function = CheckedFunctionDecl {
            id: f.id,
            identifier: f.identifier.clone(),
            params: function_params,
            return_type: checked_return_type,
            is_exported: false,
            body: FunctionBodyKind::External,
        };

        self.program
            .declarations
            .insert(f.id, CheckedDeclaration::Function(function));
        self.current_scope.map_name_to_decl(f.identifier.name, f.id);
    }
}
