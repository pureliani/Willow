use crate::ast::Span;
use crate::compile::interner::StringId;
use crate::hir::errors::{SemanticError, SemanticErrorKind};
use crate::hir::types::checked_type::Type;
use crate::hir::utils::numeric::{
    get_numeric_type_rank, is_float, is_integer, is_signed,
};

pub enum AdjustmentError {
    Incompatible,
    TryExplicitCast,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Adjustment {
    Identity,

    SExt,   // Sign Extend
    ZExt,   // Zero Extend
    Trunc,  // Truncate
    FExt,   // Float Extend
    FTrunc, // Float Truncate
    SIToF,  // Signed Int To Float
    UIToF,  // Unsigned Int To Float
    FToSI,  // Float To Signed Int
    FToUI,  // Float To Unsigned Int

    WrapInUnion(usize),
    UnwrapUnion,

    /// Maps old variant indices to new variant indices, Vec<(old_idx, new_idx)>
    ReTagUnion(Vec<(u64, u64)>),

    CoerceStruct {
        field_adjustments: Vec<(StringId, Adjustment)>,
    },
}

/// Computes the adjustment needed to convert `source_type` to `target_type`
pub fn compute_type_adjustment(
    source: &Type,
    target: &Type,
    is_explicit: bool,
) -> Result<Adjustment, AdjustmentError> {
    if source == target {
        return Ok(Adjustment::Identity);
    }

    let source = if let Type::Literal(lit) = source {
        lit.widen()
    } else {
        source
    };

    if source == target {
        return Ok(Adjustment::Identity);
    }

    if is_integer(source) && is_integer(target) {
        let s_rank = get_numeric_type_rank(source).unwrap();
        let t_rank = get_numeric_type_rank(target).unwrap();

        if t_rank > s_rank {
            return if is_signed(source) {
                Ok(Adjustment::SExt)
            } else {
                Ok(Adjustment::ZExt)
            };
        } else if t_rank < s_rank && is_explicit {
            return Ok(Adjustment::Trunc);
        }
    }

    if is_float(source) && is_float(target) {
        let s_rank = get_numeric_type_rank(source).unwrap();
        let t_rank = get_numeric_type_rank(target).unwrap();

        if t_rank > s_rank {
            return Ok(Adjustment::FExt);
        } else if t_rank < s_rank && is_explicit {
            return Ok(Adjustment::FTrunc);
        }
    }

    if is_integer(source) && is_float(target) {
        return if is_signed(source) {
            Ok(Adjustment::SIToF)
        } else {
            Ok(Adjustment::UIToF)
        };
    }

    if is_float(source) && is_integer(target) && is_explicit {
        return if is_signed(target) {
            Ok(Adjustment::FToSI)
        } else {
            Ok(Adjustment::FToUI)
        };
    }

    if let Type::Union { narrowed, .. } = source {
        if narrowed.len() == 1 && narrowed.contains(target) {
            return Ok(Adjustment::UnwrapUnion);
        }
    }

    if let (Some(source_narrowed), Some(target_narrowed)) = (
        source.get_narrowed_variants(),
        target.get_narrowed_variants(),
    ) {
        let mut all_mapped = true;
        for sv in source_narrowed {
            if !target_narrowed.contains(sv) {
                all_mapped = false;
                break;
            }
        }

        if all_mapped {
            let source_base = source.get_base_variants().unwrap();
            let target_base = target.get_base_variants().unwrap();
            let mut mapping = Vec::new();

            for (old_idx, sv) in source_base.iter().enumerate() {
                if let Some(new_idx) = target_base.iter().position(|tv| sv == tv) {
                    mapping.push((old_idx as u64, new_idx as u64));
                }
            }
            return Ok(Adjustment::ReTagUnion(mapping));
        }
    }

    if let Some(target_base) = target.get_base_variants() {
        if let Some(idx) = target_base.iter().position(|v| v == source) {
            if target.get_narrowed_variants().unwrap().contains(source) {
                return Ok(Adjustment::WrapInUnion(idx));
            }
        }
    }

    if let (Type::Struct(s_fields), Type::Struct(t_fields)) = (&source, target) {
        if s_fields.len() == t_fields.len() {
            let mut field_adjustments = Vec::new();
            let mut possible = true;

            for (sf, tf) in s_fields.iter().zip(t_fields.iter()) {
                if sf.identifier.name != tf.identifier.name {
                    possible = false;
                    break;
                }

                if sf.ty == tf.ty {
                    continue;
                }

                match compute_type_adjustment(&sf.ty.kind, &tf.ty.kind, is_explicit) {
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
    left: &Type,
    left_span: Span,
    right: &Type,
    right_span: Span,
) -> Result<Type, SemanticError> {
    let span = Span {
        start: left_span.start,
        end: right_span.end,
        path: left_span.path.clone(),
    };

    let left_type = if is_float(left) || is_integer(left) {
        left
    } else {
        return Err(SemanticError {
            kind: SemanticErrorKind::ExpectedANumericOperand,
            span: left_span,
        });
    };

    let right_type = if is_float(right) || is_integer(right) {
        right
    } else {
        return Err(SemanticError {
            kind: SemanticErrorKind::ExpectedANumericOperand,
            span: right_span,
        });
    };

    if (is_float(left_type) && is_integer(right_type))
        || (is_integer(left_type) && is_float(right_type))
    {
        return Err(SemanticError {
            kind: SemanticErrorKind::MixedFloatAndInteger,
            span,
        });
    }

    if is_signed(left_type) != is_signed(right_type) {
        return Err(SemanticError {
            kind: SemanticErrorKind::MixedSignedAndUnsigned,
            span,
        });
    }

    if right_type == left_type {
        return Ok(left_type.clone());
    }

    let left_rank = get_numeric_type_rank(left_type);
    let right_rank = get_numeric_type_rank(right_type);

    if left_rank > right_rank {
        Ok(left_type.clone())
    } else {
        Ok(right_type.clone())
    }
}
