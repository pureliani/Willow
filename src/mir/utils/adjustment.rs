use crate::ast::Span;
use crate::compile::interner::{StringId, TypeId};
use crate::mir::builders::{Builder, BuilderContext, InBlock, ValueId};
use crate::mir::errors::{SemanticError, SemanticErrorKind};
use crate::mir::types::checked_declaration::CheckedDeclaration;
use crate::mir::types::checked_type::{LiteralType, StructKind, Type};

pub enum AdjustmentError {
    Incompatible,
    TryExplicitCast,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Adjustment {
    Identity,
    Materialize,

    SIToF,
    UIToF,
    FToSI,
    FToUI,
    FExt,
    FTrunc,
    Trunc,
    SExt,
    ZExt,
    BitCast,

    WrapInUnion,
    UnwrapUnion,

    Chain(Vec<(Adjustment, TypeId)>),

    CoerceStruct {
        field_adjustments: Vec<(StringId, Adjustment)>,
    },
}

impl<'a, C: BuilderContext> Builder<'a, C> {
    /// Computes the adjustment needed to convert `source_type` to `target_type`
    pub fn compute_type_adjustment(
        &mut self,
        source: TypeId,
        target: TypeId,
        is_explicit: bool,
    ) -> Result<Adjustment, AdjustmentError> {
        if source == target
            || source == self.types.unknown()
            || target == self.types.unknown()
        {
            return Ok(Adjustment::Identity);
        };

        let src_ty = self.types.resolve(source);
        let tgt_ty = self.types.resolve(target);

        if let Type::GenericParam {
            identifier: t_id, ..
        } = tgt_ty
        {
            if let Type::GenericParam {
                identifier: s_id, ..
            } = src_ty
            {
                if s_id.name == t_id.name {
                    return Ok(Adjustment::Identity);
                }
            }
            return Err(AdjustmentError::Incompatible);
        }

        if let Type::GenericParam {
            extends: Some(c), ..
        } = src_ty
        {
            return self.compute_type_adjustment(c, target, is_explicit);
        }

        if let Type::Literal(lt) = src_ty {
            if target == self.types.widen_literal(lt) {
                return Ok(Adjustment::Materialize);
            }
        }

        if let Type::Literal(LiteralType::Fn(decl_id)) = src_ty {
            if let Type::IndirectFn(ref target_sig) = tgt_ty {
                let source_decl = self.program.declarations.get(&decl_id).unwrap();
                if let CheckedDeclaration::Function(f) = source_decl {
                    if f.return_type.id == target_sig.return_type.id
                        && f.params.len() == target_sig.params.len()
                    {
                        let mut params_match = true;
                        for (sp, tp) in f.params.iter().zip(target_sig.params.iter()) {
                            if sp.ty.id != tp.ty.id {
                                params_match = false;
                                break;
                            }
                        }
                        if params_match {
                            return Ok(Adjustment::Materialize);
                        }
                    }
                }
            }
        }

        if self.types.is_integer(source) && self.types.is_integer(target) {
            let s_rank = self.types.get_numeric_type_rank(source).unwrap();
            let t_rank = self.types.get_numeric_type_rank(target).unwrap();

            if t_rank > s_rank {
                return if self.types.is_signed(source) {
                    Ok(Adjustment::SExt)
                } else {
                    Ok(Adjustment::ZExt)
                };
            } else if t_rank < s_rank && is_explicit {
                return Ok(Adjustment::Trunc);
            } else {
                if is_explicit {
                    return Ok(Adjustment::BitCast);
                }
            }
        }

        if self.types.is_float(source) && self.types.is_float(target) {
            let s_rank = self.types.get_numeric_type_rank(source).unwrap();
            let t_rank = self.types.get_numeric_type_rank(target).unwrap();

            if t_rank > s_rank {
                return Ok(Adjustment::FExt);
            } else if t_rank < s_rank && is_explicit {
                return Ok(Adjustment::FTrunc);
            }
        }

        if self.types.is_integer(source) && self.types.is_float(target) {
            let (src, tgt) = (src_ty, tgt_ty);
            let is_lossless = matches!(
                (src, tgt),
                (
                    Type::I8 | Type::U8 | Type::I16 | Type::U16,
                    Type::F32 | Type::F64
                ) | (Type::I32 | Type::U32, Type::F64)
            );

            if is_lossless || is_explicit {
                return if self.types.is_signed(source) {
                    Ok(Adjustment::SIToF)
                } else {
                    Ok(Adjustment::UIToF)
                };
            } else {
                return Err(AdjustmentError::TryExplicitCast);
            }
        }

        if self.types.is_float(source) && self.types.is_integer(target) && is_explicit {
            return if self.types.is_signed(target) {
                Ok(Adjustment::FToSI)
            } else {
                Ok(Adjustment::FToUI)
            };
        }

        if let (Some(source_variants), Some(target_variants)) = (
            self.types.get_union_variants(source),
            self.types.get_union_variants(target),
        ) {
            for sv in source_variants.iter() {
                if !target_variants.contains(sv) {
                    return Err(AdjustmentError::Incompatible);
                }
            }
        }

        if let Some(target_variants) = self.types.get_union_variants(target) {
            if target_variants.contains(&source) {
                return Ok(Adjustment::WrapInUnion);
            }
            if let Type::Literal(lt) = src_ty {
                let widened_ty = self.types.widen_literal(lt);
                if target_variants.contains(&widened_ty) {
                    return Ok(Adjustment::Chain(vec![
                        (Adjustment::Materialize, widened_ty),
                        (Adjustment::WrapInUnion, target),
                    ]));
                }
            }
        }

        if let Some(source_variants) = self.types.get_union_variants(source) {
            if source_variants.len() == 1 && source_variants.contains(&target) {
                return Ok(Adjustment::UnwrapUnion);
            }
        }

        if let (
            Type::Struct(StructKind::UserDefined(s_fields)),
            Type::Struct(StructKind::UserDefined(t_fields)),
        ) = (src_ty, tgt_ty)
        {
            if s_fields.len() == t_fields.len() {
                let mut field_adjustments = Vec::new();
                let mut possible = true;

                for (sf, tf) in s_fields.iter().zip(t_fields.iter()) {
                    if sf.identifier.name != tf.identifier.name {
                        possible = false;
                        break;
                    }

                    match self.compute_type_adjustment(sf.ty.id, tf.ty.id, is_explicit) {
                        Ok(adj) => {
                            if adj != Adjustment::Identity {
                                field_adjustments.push((sf.identifier.name, adj));
                            }
                        }
                        Err(_) => {
                            possible = false;
                            break;
                        }
                    }
                }

                if possible {
                    if field_adjustments.is_empty() {
                        return Ok(Adjustment::Identity);
                    }
                    if is_explicit {
                        return Ok(Adjustment::CoerceStruct { field_adjustments });
                    } else {
                        return Err(AdjustmentError::TryExplicitCast);
                    }
                }
            }
        }

        Err(AdjustmentError::Incompatible)
    }

    pub fn arithmetic_supertype(
        &self,
        left: TypeId,
        left_span: Span,
        right: TypeId,
        right_span: Span,
    ) -> Result<TypeId, SemanticError> {
        let span = Span {
            start: left_span.start,
            end: right_span.end,
            path: left_span.path.clone(),
        };

        if left == self.types.unknown() || right == self.types.unknown() {
            return Ok(self.types.unknown());
        }

        let effective_left = self.types.unwrap_generic_bound(left);
        let effective_right = self.types.unwrap_generic_bound(right);

        let left_type = if self.types.is_float(effective_left)
            || self.types.is_integer(effective_left)
        {
            effective_left
        } else {
            return Err(SemanticError {
                kind: SemanticErrorKind::ExpectedANumericOperand {
                    received: effective_left,
                },
                span: left_span,
            });
        };

        let right_type = if self.types.is_float(effective_right)
            || self.types.is_integer(effective_right)
        {
            effective_right
        } else {
            return Err(SemanticError {
                kind: SemanticErrorKind::ExpectedANumericOperand {
                    received: effective_right,
                },
                span: right_span,
            });
        };

        if (self.types.is_float(left_type) && self.types.is_integer(right_type))
            || (self.types.is_integer(left_type) && self.types.is_float(right_type))
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::MixedFloatAndInteger,
                span,
            });
        }

        if self.types.is_signed(left_type) != self.types.is_signed(right_type) {
            return Err(SemanticError {
                kind: SemanticErrorKind::MixedSignedAndUnsigned,
                span,
            });
        }

        if right_type == left_type {
            return Ok(left_type);
        }

        let left_rank = self.types.get_numeric_type_rank(left_type);
        let right_rank = self.types.get_numeric_type_rank(right_type);

        if left_rank > right_rank {
            Ok(left_type)
        } else {
            Ok(right_type)
        }
    }
}

impl<'a> Builder<'a, InBlock> {
    pub fn compute_adjustment(
        &mut self,
        source: ValueId,
        target: TypeId,
        is_explicit: bool,
    ) -> Result<Adjustment, SemanticErrorKind> {
        let source_type = self.get_value_type(source);

        self.compute_type_adjustment(source_type, target, is_explicit)
            .map_err(|err| match err {
                AdjustmentError::Incompatible => SemanticErrorKind::CannotCastType {
                    source_type,
                    target_type: target,
                },
                AdjustmentError::TryExplicitCast => SemanticErrorKind::TryExplicitCast,
            })
    }

    /// Applies a previously computed adjustment, emitting the necessary
    /// instructions and returning the adjusted value.
    pub fn apply_adjustment(
        &mut self,
        source: ValueId,
        adjustment: Adjustment,
        target_type: TypeId,
        _span: Span,
    ) -> ValueId {
        match adjustment {
            Adjustment::Identity => source,

            Adjustment::SExt => self.emit_sext(source, target_type),
            Adjustment::ZExt => self.emit_zext(source, target_type),
            Adjustment::Trunc => self.emit_trunc(source, target_type),
            Adjustment::FExt => self.emit_fext(source, target_type),
            Adjustment::FTrunc => self.emit_ftrunc(source, target_type),
            Adjustment::SIToF => self.emit_sitof(source, target_type),
            Adjustment::UIToF => self.emit_uitof(source, target_type),
            Adjustment::FToSI => self.emit_ftosi(source, target_type),
            Adjustment::FToUI => self.emit_ftoui(source, target_type),
            Adjustment::BitCast => self.emit_bitcast(source, target_type),

            Adjustment::WrapInUnion => {
                let target_variants = self.types.get_union_variants(target_type).unwrap();
                self.emit_wrap_in_union(source, &target_variants)
            }
            Adjustment::UnwrapUnion => self.emit_unwrap_from_union(source, target_type),
            Adjustment::Chain(adjustments) => {
                let mut current = source;
                for (adj, ty) in adjustments {
                    current = self.apply_adjustment(current, adj, ty, _span.clone());
                }
                current
            }
            Adjustment::CoerceStruct {
                field_adjustments: _,
            } => {
                // self.apply_struct_coercion(source, target_type, field_adjustments, span)
                todo!("Struct coercion")
            }
            Adjustment::Materialize => {
                let source_ty = self.get_value_type(source);
                if let Type::Literal(lit_ty) = self.types.resolve(source_ty) {
                    self.emit_materialize(lit_ty)
                } else {
                    panic!("INTERNAL COMPILER ERROR: Materialize adjustment applied to non-literal type");
                }
            }
        }
    }
}
