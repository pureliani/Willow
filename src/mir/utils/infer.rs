use std::collections::{BTreeSet, HashMap};

use crate::{
    ast::{
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        SymbolId,
    },
    compile::interner::{StringId, TypeId},
    mir::{
        builders::{Builder, BuilderContext},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, GenericDeclaration},
            checked_type::{LiteralType, StructKind, Type},
        },
        utils::scope::ScopeKind,
    },
};

impl<'a, C: BuilderContext> Builder<'a, C> {
    pub fn contains_generic(&self, ast: &TypeAnnotation) -> bool {
        match &ast.kind {
            TypeAnnotationKind::Identifier(id) => {
                matches!(
                    self.current_scope.lookup(id.name),
                    Some(SymbolId::GenericParameter(_))
                )
            }
            TypeAnnotationKind::List(inner) => self.contains_generic(inner),
            TypeAnnotationKind::Struct(fields) => {
                fields.iter().any(|f| self.contains_generic(&f.constraint))
            }
            TypeAnnotationKind::Union(variants) => {
                variants.iter().any(|v| self.contains_generic(v))
            }
            TypeAnnotationKind::FnType {
                params,
                return_type,
            } => {
                params.iter().any(|p| self.contains_generic(&p.constraint))
                    || self.contains_generic(return_type)
            }
            TypeAnnotationKind::GenericApply { left, args } => {
                self.contains_generic(left)
                    || args.iter().any(|a| self.contains_generic(a))
            }
            _ => false,
        }
    }

    /// Infers generic type arguments by comparing an expected AST type annotation
    /// against an actual resolved TypeId
    pub fn infer_type_arguments(
        &mut self,
        expected_ast: &TypeAnnotation,
        actual_ty: TypeId,
        inferred: &mut HashMap<StringId, TypeId>,
    ) -> Result<(), SemanticError> {
        if actual_ty == self.types.unknown() {
            return Ok(());
        }

        match &expected_ast.kind {
            TypeAnnotationKind::Identifier(id) => {
                if let Some(SymbolId::GenericParameter(name)) =
                    self.current_scope.lookup(id.name)
                {
                    if let Some(&existing_ty) = inferred.get(&name) {
                        if existing_ty != actual_ty {
                            return Err(SemanticError {
                                span: id.span.clone(),
                                kind: SemanticErrorKind::ConflictingGenericBinding {
                                    param_name: id.clone(),
                                    expected: existing_ty,
                                    received: actual_ty,
                                },
                            });
                        }
                    } else {
                        inferred.insert(name, actual_ty);
                    }
                }
            }
            TypeAnnotationKind::Union(expected_variants) => {
                let mut generic_asts = Vec::new();
                let mut concrete_asts = Vec::new();

                for variant in expected_variants {
                    if self.contains_generic(variant) {
                        generic_asts.push(variant);
                    } else {
                        concrete_asts.push(variant);
                    }
                }

                if generic_asts.len() > 1 {
                    return Err(SemanticError {
                        span: expected_ast.span.clone(),
                        kind: SemanticErrorKind::AmbiguousGenericInference,
                    });
                }

                if let Some(generic_ast) = generic_asts.pop() {
                    let empty_subs = HashMap::new();
                    let mut concrete_set = BTreeSet::new();

                    for c_ast in concrete_asts {
                        let c_ty = self.check_type_annotation(c_ast, &empty_subs).id;
                        if let Some(vars) = self.types.get_union_variants(c_ty) {
                            concrete_set.extend(vars);
                        } else {
                            concrete_set.insert(c_ty);
                        }
                    }

                    let actual_set =
                        if let Some(vars) = self.types.get_union_variants(actual_ty) {
                            vars
                        } else {
                            BTreeSet::from([actual_ty])
                        };

                    let remainder: Vec<TypeId> =
                        actual_set.difference(&concrete_set).copied().collect();

                    if !remainder.is_empty() {
                        let remainder_ty = self.types.make_union(remainder);
                        self.infer_type_arguments(generic_ast, remainder_ty, inferred)?;
                    }
                }
            }
            TypeAnnotationKind::List(inner_ast) => {
                let actual_resolved = self.types.resolve(actual_ty);
                if let Type::Pointer(inner_ptr) = actual_resolved {
                    if let Type::Struct(StructKind::ListHeader(elem_ty)) =
                        self.types.resolve(inner_ptr)
                    {
                        self.infer_type_arguments(inner_ast, elem_ty, inferred)?;
                    }
                }
            }
            TypeAnnotationKind::Struct(expected_fields) => {
                let actual_resolved = self.types.resolve(actual_ty);

                let struct_ty = if let Type::Pointer(inner) = actual_resolved {
                    inner
                } else {
                    actual_ty
                };

                if let Type::Struct(StructKind::UserDefined(actual_fields)) =
                    self.types.resolve(struct_ty)
                {
                    for expected_field in expected_fields {
                        if let Some(actual_field) = actual_fields
                            .iter()
                            .find(|f| f.identifier.name == expected_field.identifier.name)
                        {
                            self.infer_type_arguments(
                                &expected_field.constraint,
                                actual_field.ty.id,
                                inferred,
                            )?;
                        }
                    }
                }
            }
            TypeAnnotationKind::FnType {
                params: expected_params,
                return_type: expected_ret,
            } => {
                let actual_resolved = self.types.resolve(actual_ty);

                let signature = match actual_resolved {
                    Type::IndirectFn(f) => {
                        let p_tys: Vec<TypeId> =
                            f.params.into_iter().map(|p| p.ty.id).collect();
                        Some((p_tys, f.return_type.id))
                    }
                    Type::Literal(LiteralType::Fn(decl_id)) => {
                        if let Some(CheckedDeclaration::Function(f)) =
                            self.program.declarations.get(&decl_id)
                        {
                            let p_tys: Vec<TypeId> =
                                f.params.iter().map(|p| p.ty.id).collect();
                            Some((p_tys, f.return_type.id))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some((act_params, act_ret)) = signature {
                    if expected_params.len() == act_params.len() {
                        for (exp_p, act_p_ty) in expected_params.iter().zip(act_params) {
                            self.infer_type_arguments(
                                &exp_p.constraint,
                                act_p_ty,
                                inferred,
                            )?;
                        }
                        self.infer_type_arguments(expected_ret, act_ret, inferred)?;
                    }
                }
            }
            TypeAnnotationKind::GenericApply { left, args } => {
                if let TypeAnnotationKind::Identifier(id) = &left.kind {
                    if let Some(SymbolId::Generic(gen_id)) =
                        self.current_scope.lookup(id.name)
                    {
                        let generic_decl = self
                            .program
                            .generic_declarations
                            .get(&gen_id)
                            .unwrap()
                            .clone();

                        if let GenericDeclaration::TypeAlias { decl, decl_scope } =
                            generic_decl
                        {
                            if decl.generic_params.len() == args.len() {
                                let caller_scope = self.current_scope.clone();
                                self.current_scope = decl_scope.enter(
                                    ScopeKind::GenericParams,
                                    expected_ast.span.start,
                                );
                                for param in &decl.generic_params {
                                    self.current_scope.map_name_to_symbol(
                                        param.identifier.name,
                                        SymbolId::GenericParameter(param.identifier.name),
                                    );
                                }

                                let mut temp_inferred = HashMap::new();
                                let infer_result = self.infer_type_arguments(
                                    &decl.value,
                                    actual_ty,
                                    &mut temp_inferred,
                                );

                                self.current_scope = caller_scope;

                                if infer_result.is_ok() {
                                    for (param, arg_ast) in
                                        decl.generic_params.iter().zip(args.iter())
                                    {
                                        if let Some(&inferred_ty) =
                                            temp_inferred.get(&param.identifier.name)
                                        {
                                            self.infer_type_arguments(
                                                arg_ast,
                                                inferred_ty,
                                                inferred,
                                            )?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
