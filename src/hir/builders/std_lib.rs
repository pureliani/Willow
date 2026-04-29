use crate::{
    ast::{stmt::ImportItem, IdentifierNode, Span, SymbolId},
    compile::interner::TypeId,
    globals::{next_declaration_id, STRING_INTERNER},
    hir::{
        builders::{
            Builder, CheckedFunctionDecl, FunctionBodyKind, FunctionParam, InModule,
        },
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::CheckedDeclaration,
            checked_type::{SpannedType, StructKind, Type},
        },
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
                            .insert(decl_id, CheckedDeclaration::Function(func));

                        let name_to_bind = alias.unwrap_or(identifier).name;
                        self.current_scope.map_name_to_symbol(
                            name_to_bind,
                            SymbolId::Concrete(decl_id),
                        );
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
    ) -> Option<CheckedFunctionDecl> {
        let void_ty = self.types.void();
        let string_ty = self
            .types
            .ptr(self.types.intern(&Type::Struct(StructKind::String)));
        let usize_ty = self.types.usize(None);

        match (module, symbol) {
            // std/io
            ("std/io", "print") => {
                Some(self.make_builtin_fn(symbol, span, vec![("s", string_ty)], void_ty))
            }

            // std/string
            ("std/string", "concat") => {
                let ptr_ptr_string = self.types.ptr(string_ty);
                Some(self.make_builtin_fn(
                    symbol,
                    span,
                    vec![("parts", ptr_ptr_string), ("count", usize_ty)],
                    string_ty,
                ))
            }

            // std/fs
            ("std/fs", "read_file") => Some(self.make_builtin_fn(
                symbol,
                span,
                vec![("path", string_ty)],
                string_ty,
            )),

            _ => None,
        }
    }

    fn make_builtin_fn(
        &self,
        name: &str,
        span: Span,
        params: Vec<(&str, TypeId)>,
        return_type: TypeId,
    ) -> CheckedFunctionDecl {
        let id = next_declaration_id();
        let identifier = IdentifierNode {
            name: STRING_INTERNER.intern(name),
            span: span.clone(),
        };

        let function_params = params
            .into_iter()
            .map(|(p_name, p_ty)| FunctionParam {
                identifier: IdentifierNode {
                    name: STRING_INTERNER.intern(p_name),
                    span: span.clone(),
                },
                ty: SpannedType {
                    id: p_ty,
                    span: span.clone(),
                },
                decl_id: None,
                value_id: None,
            })
            .collect();

        CheckedFunctionDecl {
            id,
            identifier,
            params: function_params,
            return_type: SpannedType {
                id: return_type,
                span: span.clone(),
            },
            is_exported: false,
            body: FunctionBodyKind::External,
        }
    }
}
