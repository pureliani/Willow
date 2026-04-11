use std::collections::HashMap;

use crate::{
    ast::{DeclarationId, GenericDeclarationId, Span, SymbolId},
    compile::interner::TypeId,
    globals::next_declaration_id,
    mir::{
        builders::{
            Builder, CheckedFunctionDecl, FunctionBodyKind, FunctionParam, InBlock,
        },
        types::checked_declaration::{CheckedDeclaration, GenericDeclaration},
        utils::scope::ScopeKind,
    },
};

impl<'a> Builder<'a, InBlock> {
    /// Takes a generic function template and a list of concrete type arguments,
    /// and generates a fully monomorphized concrete function (or returns a cached one)
    pub fn monomorphize_function(
        &mut self,
        gen_id: GenericDeclarationId,
        type_args: Vec<TypeId>,
        span: Span,
    ) -> DeclarationId {
        let cache_key = (gen_id, type_args.clone());

        if let Some(&concrete_id) = self.program.monomorphizations.get(&cache_key) {
            return concrete_id;
        }

        let generic_decl = self
            .program
            .generic_declarations
            .get(&gen_id)
            .unwrap()
            .clone();
        let (fn_decl, decl_scope) = match generic_decl {
            GenericDeclaration::Function { decl, decl_scope } => (decl, decl_scope),
            _ => panic!("INTERNAL COMPILER ERROR: Expected GenericDeclaration::Function"),
        };

        let mut inner_substitutions = HashMap::new();
        for (param, arg_ty) in fn_decl.generic_params.iter().zip(type_args.iter()) {
            inner_substitutions.insert(param.identifier.name, *arg_ty);
        }

        let new_decl_id = next_declaration_id();
        self.program
            .monomorphizations
            .insert(cache_key, new_decl_id);

        let caller_scope = self.current_scope.clone();
        self.current_scope = decl_scope.enter(ScopeKind::GenericParams, span.start);
        for param in &fn_decl.generic_params {
            self.current_scope.map_name_to_symbol(
                param.identifier.name,
                SymbolId::GenericParameter(param.identifier.name),
            );
        }

        let checked_params = self.check_params(&fn_decl.params, &inner_substitutions);
        let checked_return_type =
            self.check_type_annotation(&fn_decl.return_type, &inner_substitutions);

        let function_params = checked_params
            .into_iter()
            .map(|p| FunctionParam {
                identifier: p.identifier,
                ty: p.ty,
                decl_id: None,
                value_id: None,
            })
            .collect();

        let concrete_func = CheckedFunctionDecl {
            id: new_decl_id,
            identifier: fn_decl.identifier.clone(),
            params: function_params,
            return_type: checked_return_type,
            is_exported: false,
            body: FunctionBodyKind::NotBuilt,
        };

        self.program
            .declarations
            .insert(new_decl_id, CheckedDeclaration::Function(concrete_func));

        let mut concrete_ast = fn_decl.clone();
        concrete_ast.id = new_decl_id;

        if let Err(e) = self
            .as_module()
            .build_fn_body(concrete_ast, &inner_substitutions)
        {
            self.errors.push(e);
        }

        self.current_scope = caller_scope;

        new_decl_id
    }
}
