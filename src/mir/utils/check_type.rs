use std::collections::HashMap;

use crate::{
    ast::{
        decl::Param,
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        IdentifierNode, SymbolId,
    },
    compile::interner::{StringId, TypeId},
    mir::{
        builders::{Builder, BuilderContext},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{
                CheckedDeclaration, CheckedParam, FnType, GenericDeclaration,
            },
            checked_type::{SpannedType, StructKind, Type},
            ordered_float::{OrderedF32, OrderedF64},
        },
        utils::layout::pack_struct,
    },
};

impl<'a, C: BuilderContext> Builder<'a, C> {
    pub fn check_params(
        &mut self,
        params: &[Param],
        substitutions: &HashMap<StringId, TypeId>,
    ) -> Vec<CheckedParam> {
        params
            .iter()
            .map(|p| CheckedParam {
                ty: self.check_type_annotation(&p.constraint, substitutions),
                identifier: p.identifier.clone(),
            })
            .collect()
    }

    pub fn check_type_identifier_annotation(&mut self, id: IdentifierNode) -> TypeId {
        match self.current_scope.lookup(id.name) {
            Some(SymbolId::Concrete(decl_id)) => {
                match self.program.declarations.get(&decl_id).unwrap_or_else(|| {
                    panic!(
                        "INTERNAL COMPILER ERROR: Expected declarations to contain \
                         DeclarationId({}) key",
                        decl_id.0
                    )
                }) {
                    CheckedDeclaration::TypeAlias(decl) => decl.value.id,
                    _ => {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::IdentifierIsNotAType(id.clone()),
                            span: id.span.clone(),
                        });
                        self.types.unknown()
                    }
                }
            }
            Some(SymbolId::Generic(_)) => {
                self.errors.push(SemanticError {
                    kind: SemanticErrorKind::MissingGenericArguments,
                    span: id.span.clone(),
                });
                self.types.unknown()
            }
            None => {
                self.errors.push(SemanticError {
                    span: id.span.clone(),
                    kind: SemanticErrorKind::UndeclaredType(id),
                });
                self.types.unknown()
            }
        }
    }

    pub fn check_type_annotation(
        &mut self,
        annotation: &TypeAnnotation,
        substitutions: &HashMap<StringId, TypeId>,
    ) -> SpannedType {
        let id = match &annotation.kind {
            TypeAnnotationKind::Void => self.types.void(),
            TypeAnnotationKind::Null => self.types.null(),
            TypeAnnotationKind::Bool(lit) => self.types.bool(*lit),
            TypeAnnotationKind::U8(lit) => self.types.u8(*lit),
            TypeAnnotationKind::U16(lit) => self.types.u16(*lit),
            TypeAnnotationKind::U32(lit) => self.types.u32(*lit),
            TypeAnnotationKind::U64(lit) => self.types.u64(*lit),
            TypeAnnotationKind::USize(lit) => self.types.usize(*lit),
            TypeAnnotationKind::I8(lit) => self.types.i8(*lit),
            TypeAnnotationKind::I16(lit) => self.types.i16(*lit),
            TypeAnnotationKind::I32(lit) => self.types.i32(*lit),
            TypeAnnotationKind::I64(lit) => self.types.i64(*lit),
            TypeAnnotationKind::ISize(lit) => self.types.isize(*lit),
            TypeAnnotationKind::F32(lit) => self.types.f32(lit.map(OrderedF32)),
            TypeAnnotationKind::F64(lit) => self.types.f64(lit.map(OrderedF64)),
            TypeAnnotationKind::String(lit) => self.types.string(*lit),
            TypeAnnotationKind::Identifier(id) => {
                if let Some(&ty) = substitutions.get(&id.name) {
                    ty
                } else {
                    self.check_type_identifier_annotation(id.clone())
                }
            }
            TypeAnnotationKind::FnType {
                params,
                return_type,
            } => {
                let checked_params = self.check_params(params, substitutions);
                let checked_return_type =
                    self.check_type_annotation(return_type, substitutions);

                self.types.intern(&Type::IndirectFn(FnType {
                    params: checked_params,
                    return_type: checked_return_type,
                }))
            }
            TypeAnnotationKind::Union(variants) => {
                let mut checked_variants = Vec::new();

                for v in variants {
                    checked_variants
                        .push(self.check_type_annotation(v, substitutions).id);
                }

                self.types.make_union(checked_variants)
            }
            TypeAnnotationKind::GenericApply { left, args } => {
                if let TypeAnnotationKind::Identifier(id) = &left.kind {
                    match self.current_scope.lookup(id.name) {
                        Some(SymbolId::Generic(gen_id)) => {
                            let generic_decl = self
                                .program
                                .generic_declarations
                                .get(&gen_id)
                                .unwrap()
                                .clone();

                            match generic_decl {
                                GenericDeclaration::TypeAlias { decl, decl_scope } => {
                                    let expected_arg_count = decl.generic_params.len();
                                    let received_arg_count = args.len();

                                    if expected_arg_count != received_arg_count {
                                        self.errors.push(SemanticError {
                                            span: annotation.span.clone(),
                                            kind: SemanticErrorKind::GenericArgumentCountMismatch {
                                                expected: expected_arg_count,
                                                received: received_arg_count,
                                            },
                                        });
                                        self.types.unknown()
                                    } else {
                                        let mut evaluated_args = Vec::new();
                                        for arg in args {
                                            evaluated_args.push(
                                                self.check_type_annotation(
                                                    arg,
                                                    substitutions,
                                                )
                                                .id,
                                            );
                                        }

                                        let mut inner_substitutions = HashMap::new();
                                        for (param, arg_ty) in
                                            decl.generic_params.iter().zip(evaluated_args)
                                        {
                                            inner_substitutions
                                                .insert(param.identifier.name, arg_ty);
                                        }

                                        let caller_scope = self.current_scope.clone();
                                        self.current_scope = decl_scope;

                                        let result_id = self
                                            .check_type_annotation(
                                                &decl.value,
                                                &inner_substitutions,
                                            )
                                            .id;

                                        self.current_scope = caller_scope;

                                        result_id
                                    }
                                }
                                GenericDeclaration::Function { .. } => {
                                    self.errors.push(SemanticError {
                                        span: id.span.clone(),
                                        kind: SemanticErrorKind::IdentifierIsNotAType(
                                            id.clone(),
                                        ),
                                    });
                                    self.types.unknown()
                                }
                            }
                        }
                        Some(SymbolId::Concrete(_)) => {
                            self.errors.push(SemanticError {
                                span: left.span.clone(),
                                kind: SemanticErrorKind::CannotApplyTypeArguments,
                            });
                            self.types.unknown()
                        }
                        None => {
                            self.errors.push(SemanticError {
                                span: id.span.clone(),
                                kind: SemanticErrorKind::UndeclaredType(id.clone()),
                            });
                            self.types.unknown()
                        }
                    }
                } else {
                    self.errors.push(SemanticError {
                        span: left.span.clone(),
                        kind: SemanticErrorKind::CannotApplyTypeArguments,
                    });
                    self.types.unknown()
                }
            }

            TypeAnnotationKind::List(item_type) => {
                let checked_item_type =
                    self.check_type_annotation(item_type, substitutions);
                let inner = self
                    .types
                    .intern(&Type::Struct(StructKind::ListHeader(checked_item_type.id)));

                self.types.ptr(inner)
            }
            TypeAnnotationKind::Struct(items) => {
                let checked_field_types = self.check_params(items, substitutions);
                let packed = pack_struct(
                    StructKind::UserDefined(checked_field_types),
                    self.types,
                    self.program.target_ptr_size,
                    self.program.target_ptr_align,
                );
                let inner = self.types.intern(&Type::Struct(packed));
                self.types.ptr(inner)
            }
        };

        SpannedType {
            id,
            span: annotation.span.clone(),
        }
    }
}
