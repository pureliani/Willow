use crate::{
    ast::{
        decl::{Declaration, ExternFnDecl, Param},
        stmt::ImportItem,
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        IdentifierNode, Span,
    },
    globals::{next_declaration_id, STRING_INTERNER},
    hir::{
        builders::{Builder, InModule},
        errors::{SemanticError, SemanticErrorKind},
    },
};

impl<'a> Builder<'a, InModule> {
    pub fn build_std_import(
        &mut self,
        module_path: &str,
        items: Vec<ImportItem>,
        _span: Span,
    ) {
        for item in items {
            match item {
                ImportItem::Symbol { identifier, alias } => {
                    let symbol_name = STRING_INTERNER.resolve(identifier.name);

                    if let Some(func) = self.get_std_function(
                        module_path,
                        &symbol_name,
                        identifier.span.clone(),
                    ) {
                        let decl_id = func.id;

                        self.program
                            .declarations
                            .insert(decl_id, Declaration::ExternFn(func));

                        let name_to_bind = alias.unwrap_or(identifier).name;
                        self.current_scope.map_name_to_symbol(name_to_bind, decl_id);
                    } else {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::UndeclaredIdentifier(
                                identifier.clone(),
                            ),
                            span: identifier.span,
                        });
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

    fn get_std_function(
        &self,
        module: &str,
        symbol: &str,
        span: Span,
    ) -> Option<ExternFnDecl> {
        let void_ty = TypeAnnotation {
            kind: TypeAnnotationKind::Void,
            span: span.clone(),
        };

        let string_ty = TypeAnnotation {
            kind: TypeAnnotationKind::String(None),
            span: span.clone(),
        };

        let usize_ty = TypeAnnotation {
            kind: TypeAnnotationKind::USize(None),
            span: span.clone(),
        };

        match (module, symbol) {
            // std/io
            ("std/io", "print") => {
                Some(self.make_builtin_fn(symbol, span, vec![("s", string_ty)], void_ty))
            }

            // std/string
            ("std/string", "concat") => {
                let list_of_strings = TypeAnnotation {
                    kind: TypeAnnotationKind::List(Box::new(string_ty.clone())),
                    span: span.clone(),
                };

                Some(self.make_builtin_fn(
                    symbol,
                    span,
                    vec![("parts", list_of_strings), ("count", usize_ty)],
                    string_ty,
                ))
            }

            // std/fs
            ("std/fs", "read_file") => Some(self.make_builtin_fn(
                symbol,
                span,
                vec![("path", string_ty.clone())],
                string_ty,
            )),

            _ => None,
        }
    }

    fn make_builtin_fn(
        &self,
        name: &str,
        span: Span,
        params: Vec<(&str, TypeAnnotation)>,
        return_type: TypeAnnotation,
    ) -> ExternFnDecl {
        let id = next_declaration_id();
        let identifier = IdentifierNode {
            name: STRING_INTERNER.intern(name),
            span: span.clone(),
        };

        let function_params = params
            .into_iter()
            .map(|(p_name, p_ty)| Param {
                id: next_declaration_id(),
                identifier: IdentifierNode {
                    name: STRING_INTERNER.intern(p_name),
                    span: span.clone(),
                },
                constraint: p_ty,
            })
            .collect();

        ExternFnDecl {
            id,
            documentation: None,
            identifier,
            params: function_params,
            return_type,
        }
    }
}
