use std::collections::BTreeSet;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        type_annotation::TypeAnnotation,
        Span, SymbolId,
    },
    compile::interner::{GenericSubstitutions, TypeId},
    mir::{
        builders::{Builder, ConditionFact, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_type::SpannedType,
        utils::{
            facts::{narrowed_type::NarrowedTypeFact, FactSet},
            place::Place,
        },
    },
};

impl<'a> Builder<'a, InBlock> {
    fn emit_is_one_of_the_variants(
        &mut self,
        union: ValueId,
        matching_variants: &[TypeId],
        total_variants: usize,
        span: Span,
    ) -> ValueId {
        if matching_variants.is_empty() {
            return self.emit_bool(false);
        }

        if matching_variants.len() == total_variants {
            return self.emit_bool(true);
        }

        let mut iter = matching_variants.iter();
        let first_variant = iter.next().unwrap();

        let mut result_id = self.emit_test_variant(union, *first_variant);

        for variant in iter {
            let variant_clone = *variant;
            result_id = self.emit_logical_or(result_id, span.clone(), |builder| {
                builder.emit_test_variant(union, variant_clone)
            });
        }

        result_id
    }

    pub fn resolve_narrow_target(&self, expr: &Expr) -> Option<Place> {
        match &expr.kind {
            ExprKind::Identifier(ident) => {
                match self.current_scope.lookup(ident.name)? {
                    SymbolId::Concrete(decl_id) => Some(Place::Var(decl_id)),
                    SymbolId::Generic(_) | SymbolId::GenericParameter(_) => None,
                }
            }
            ExprKind::Access { left, field } => {
                let base_place = self.resolve_narrow_target(left)?;
                let base_ty = self.type_of_place(&base_place);

                let derefed_place = if self.types.is_pointer(base_ty) {
                    Place::Deref(Box::new(base_place))
                } else {
                    base_place
                };

                Some(Place::Field(Box::new(derefed_place), field.name))
            }
            _ => None,
        }
    }

    pub fn build_is_type_expr(
        &mut self,
        left: Expr,
        ty: TypeAnnotation,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let span = left.span.clone();

        let place_opt = self.resolve_narrow_target(&left);

        let current_val = self.build_expr(left, None, substitutions);
        let current_ty = self.get_value_type(current_val);

        let source_variants = match self.types.get_union_variants(current_ty) {
            Some(v) => v,
            None => {
                return self.report_error_and_get_poison(SemanticError {
                    span: span.clone(),
                    kind: SemanticErrorKind::CannotNarrowNonUnion(current_ty),
                });
            }
        };

        let target_type = self.check_type_annotation(&ty, substitutions);

        if self.types.get_union_variants(target_type.id).is_some() {
            return self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::UnsupportedUnionNarrowing,
                span: ty.span.clone(),
            });
        }

        let mut matching_variants = Vec::new();
        let mut non_matching_variants = Vec::new();

        for variant in source_variants.iter() {
            if *variant == target_type.id {
                matching_variants.push(*variant);
            } else {
                non_matching_variants.push(*variant);
            }
        }

        let result_id = self.emit_is_one_of_the_variants(
            current_val,
            &matching_variants,
            source_variants.len(),
            span.clone(),
        );

        let true_type = if !matching_variants.is_empty()
            && matching_variants.len() < source_variants.len()
        {
            Some(self.types.make_union(matching_variants.clone()))
        } else {
            None
        };

        let false_type = if !non_matching_variants.is_empty()
            && non_matching_variants.len() < source_variants.len()
        {
            Some(self.types.make_union(non_matching_variants.clone()))
        } else {
            None
        };

        if true_type.is_some() || false_type.is_some() {
            if let Some(place) = place_opt {
                let mut on_true = FactSet::new();
                if let Some(tt) = true_type {
                    let variants = self
                        .types
                        .get_union_variants(tt)
                        .unwrap_or_else(|| BTreeSet::from([tt]));
                    on_true.insert(NarrowedTypeFact { variants });
                }

                let mut on_false = FactSet::new();
                if let Some(ft) = false_type {
                    let variants = self
                        .types
                        .get_union_variants(ft)
                        .unwrap_or_else(|| BTreeSet::from([ft]));
                    on_false.insert(NarrowedTypeFact { variants });
                }

                self.condition_facts.insert(
                    result_id,
                    vec![ConditionFact {
                        place,
                        on_true,
                        on_false,
                    }],
                );
            }
        }

        self.check_expected(result_id, span, expected_type)
    }
}
