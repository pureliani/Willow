use crate::{
    ast::{
        expr::{Expr, ExprKind},
        type_annotation::TypeAnnotation,
        DeclarationId, Span,
    },
    compile::interner::StringId,
    hir::{
        builders::{Builder, InBlock, TypePredicate, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::CheckedParam,
            checked_type::{SpannedType, Type},
        },
        utils::check_type::{check_type_annotation, TypeCheckerContext},
    },
};

impl<'a> Builder<'a, InBlock> {
    fn emit_is_one_of_the_variants(
        &mut self,
        union: ValueId,
        matching_variants: &[Type],
        total_variants: usize,
        span: Span,
    ) -> ValueId {
        if matching_variants.is_empty() {
            return self.emit_bool_literal(false);
        }

        if matching_variants.len() == total_variants {
            return self.emit_bool_literal(true);
        }

        let mut iter = matching_variants.iter();
        let first_variant = iter.next().unwrap();

        let mut result_id = self.emit_test_variant(union, first_variant);

        for variant in iter {
            let variant_clone = variant.clone();
            result_id = self.emit_logical_or(result_id, span.clone(), |builder| {
                builder.emit_test_variant(union, &variant_clone)
            });
        }

        result_id
    }

    pub fn replace_field_type(
        struct_ty: &Type,
        field: StringId,
        new_field_ty: Type,
    ) -> Type {
        if let Type::Struct(fields) = struct_ty {
            let new_fields = fields
                .iter()
                .map(|f| {
                    if f.identifier.name == field {
                        CheckedParam {
                            identifier: f.identifier.clone(),
                            ty: SpannedType {
                                kind: new_field_ty.clone(),
                                span: Span::default(), // TODO: fix this later
                            },
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            Type::Struct(new_fields)
        } else {
            struct_ty.clone()
        }
    }

    /// Walk a narrowable expression up to the root variable, lifting the
    /// narrowed leaf types into the root's struct type at each level.
    pub fn resolve_narrow_target(
        &mut self,
        expr: &Expr,
        on_true: Option<Type>,
        on_false: Option<Type>,
    ) -> Option<(DeclarationId, Option<Type>, Option<Type>)> {
        match &expr.kind {
            ExprKind::Identifier(ident) => {
                let decl_id = self.current_scope.lookup(ident.name)?;
                Some((decl_id, on_true, on_false))
            }
            ExprKind::Access { left, field } => {
                let parent_val = self.build_expr(*left.clone(), None);
                let parent_ty = self.get_value_type(parent_val).clone();

                let lifted_true =
                    on_true.map(|t| Self::replace_field_type(&parent_ty, field.name, t));
                let lifted_false =
                    on_false.map(|t| Self::replace_field_type(&parent_ty, field.name, t));

                self.resolve_narrow_target(left, lifted_true, lifted_false)
            }
            _ => None,
        }
    }

    pub fn build_is_type_expr(
        &mut self,
        left: Expr,
        ty: TypeAnnotation,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let span = left.span.clone();

        let current_val = self.build_expr(left.clone(), None);
        let current_ty = self.get_value_type(current_val).clone();

        let source_variants = match current_ty.get_narrowed_variants() {
            Some(v) => v,
            None => {
                return self.report_error_and_get_poison(SemanticError {
                    span: span.clone(),
                    kind: SemanticErrorKind::CannotNarrowNonUnion(current_ty.clone()),
                });
            }
        };

        let mut type_ctx = TypeCheckerContext {
            scope: self.current_scope.clone(),
            declarations: &self.program.declarations,
            errors: self.errors,
        };
        let target_type = check_type_annotation(&mut type_ctx, &ty);

        if target_type.kind.get_narrowed_variants().is_some() {
            return self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::UnsupportedUnionNarrowing,
                span: ty.span.clone(),
            });
        }

        let mut matching_variants = Vec::new();
        let mut non_matching_variants = Vec::new();

        for variant in source_variants {
            if variant == &target_type.kind {
                matching_variants.push(variant.clone());
            } else {
                non_matching_variants.push(variant.clone());
            }
        }

        let result_id = self.emit_is_one_of_the_variants(
            current_val,
            &matching_variants,
            source_variants.len(),
            span.clone(),
        );

        let base_variants = current_ty.get_base_variants().unwrap();

        let true_type = if !matching_variants.is_empty()
            && matching_variants.len() < source_variants.len()
        {
            Some(Type::Union {
                base: base_variants.clone(),
                narrowed: matching_variants.into_iter().collect(),
            })
        } else {
            None
        };

        let false_type = if !non_matching_variants.is_empty()
            && non_matching_variants.len() < source_variants.len()
        {
            Some(Type::Union {
                base: base_variants.clone(),
                narrowed: non_matching_variants.into_iter().collect(),
            })
        } else {
            None
        };

        if true_type.is_some() || false_type.is_some() {
            if let Some((decl_id, lifted_true, lifted_false)) =
                self.resolve_narrow_target(&left, true_type, false_type)
            {
                self.type_predicates.insert(
                    result_id,
                    vec![TypePredicate {
                        decl_id,
                        on_true_type: lifted_true,
                        on_false_type: lifted_false,
                    }],
                );
            }
        }

        self.check_expected(result_id, span, expected_type)
    }
}
