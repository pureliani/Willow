use crate::ast::Span;
use crate::compile::interner::{StringId, TypeId};
use crate::mir::builders::{Builder, BuilderContext, InBlock, ValueId};
use crate::mir::errors::{SemanticError, SemanticErrorKind};
use crate::mir::types::checked_type::{StructKind, Type};

pub enum AdjustmentError {
    Incompatible,
    TryExplicitCast,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Adjustment {
    Identity,

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
        if source == target {
            return Ok(Adjustment::Identity);
        };

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
            let widened_target = self.types.widen_literal(target);
            let is_lossless = match (
                self.types.resolve(source),
                self.types.resolve(widened_target),
            ) {
                (Type::I32(Some(v)), Type::F32(_)) => (v as f32) as i32 == v,
                (Type::U32(Some(v)), Type::F32(_)) => (v as f32) as u32 == v,
                (Type::I64(Some(v)), Type::F32(_)) => (v as f32) as i64 == v,
                (Type::U64(Some(v)), Type::F32(_)) => (v as f32) as u64 == v,
                (Type::I64(Some(v)), Type::F64(_)) => (v as f64) as i64 == v,
                (Type::U64(Some(v)), Type::F64(_)) => (v as f64) as u64 == v,
                (Type::ISize(Some(v)), Type::F32(_)) => (v as f32) as isize == v,
                (Type::USize(Some(v)), Type::F32(_)) => (v as f32) as usize == v,
                (Type::ISize(Some(v)), Type::F64(_)) => (v as f64) as isize == v,
                (Type::USize(Some(v)), Type::F64(_)) => (v as f64) as usize == v,

                (src, tgt) => match (src, tgt) {
                    (
                        Type::I8(_) | Type::U8(_) | Type::I16(_) | Type::U16(_),
                        Type::F32(_) | Type::F64(_),
                    ) => true,

                    (Type::I32(_) | Type::U32(_), Type::F64(_)) => true,

                    _ => false,
                },
            };

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
            for (_old_idx, sv) in source_variants.iter().enumerate() {
                if let None = target_variants.iter().position(|tv| tv == sv) {
                    return Err(AdjustmentError::Incompatible);
                }
            }
        }

        if let Some(target_variants) = self.types.get_union_variants(target) {
            if target_variants.contains(&source) {
                return Ok(Adjustment::WrapInUnion);
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
        ) = (self.types.resolve(source), self.types.resolve(target))
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

        let left_type = if self.types.is_float(left) || self.types.is_integer(left) {
            left
        } else {
            return Err(SemanticError {
                kind: SemanticErrorKind::ExpectedANumericOperand,
                span: left_span,
            });
        };

        let right_type = if self.types.is_float(right) || self.types.is_integer(right) {
            right
        } else {
            return Err(SemanticError {
                kind: SemanticErrorKind::ExpectedANumericOperand,
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
        span: Span,
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
            Adjustment::CoerceStruct { field_adjustments } => {
                todo!()
            }
        }
    }
}
