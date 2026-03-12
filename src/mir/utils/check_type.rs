use std::collections::HashMap;

use crate::{
    ast::{
        decl::Param,
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        DeclarationId, IdentifierNode,
    },
    mir::{
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, CheckedParam, FnType},
            checked_type::{SpannedType, StructKind, Type},
            ordered_float::{OrderedF32, OrderedF64},
        },
        utils::scope::Scope,
    },
};

pub struct TypeCheckerContext<'a> {
    pub scope: Scope,
    pub declarations: &'a HashMap<DeclarationId, CheckedDeclaration>,
    pub errors: &'a mut Vec<SemanticError>,
}

pub fn check_params(ctx: &mut TypeCheckerContext, params: &[Param]) -> Vec<CheckedParam> {
    params
        .iter()
        .map(|p| CheckedParam {
            ty: check_type_annotation(ctx, &p.constraint),
            identifier: p.identifier.clone(),
        })
        .collect()
}

pub fn check_type_identifier_annotation(
    ctx: &mut TypeCheckerContext,
    id: IdentifierNode,
) -> Type {
    ctx.scope
        .lookup(id.name)
        .map(|entry| {
            match ctx.declarations.get(&entry).unwrap_or_else(|| {
                panic!(
                    "INTERNAL COMPILER ERROR: Expected declarations to contain \
                     DeclarationId({}) key",
                    entry.0
                )
            }) {
                CheckedDeclaration::TypeAlias(decl) => decl.value.kind.clone(),
                CheckedDeclaration::Function(_) => {
                    ctx.errors.push(SemanticError {
                        kind: SemanticErrorKind::CannotUseFunctionDeclarationAsType,
                        span: id.span.clone(),
                    });

                    Type::Unknown
                }
                CheckedDeclaration::Var(_) => {
                    ctx.errors.push(SemanticError {
                        kind: SemanticErrorKind::CannotUseVariableDeclarationAsType,
                        span: id.span.clone(),
                    });

                    Type::Unknown
                }
            }
        })
        .unwrap_or_else(|| {
            ctx.errors.push(SemanticError {
                span: id.span.clone(),
                kind: SemanticErrorKind::UndeclaredType(id),
            });

            Type::Unknown
        })
}

pub fn check_type_annotation(
    ctx: &mut TypeCheckerContext,
    annotation: &TypeAnnotation,
) -> SpannedType {
    let kind = match &annotation.kind {
        TypeAnnotationKind::Void => Type::Void,
        TypeAnnotationKind::Null => Type::Null,
        TypeAnnotationKind::Bool(lit) => Type::Bool(*lit),
        TypeAnnotationKind::U8(lit) => Type::U8(*lit),
        TypeAnnotationKind::U16(lit) => Type::U16(*lit),
        TypeAnnotationKind::U32(lit) => Type::U32(*lit),
        TypeAnnotationKind::U64(lit) => Type::U64(*lit),
        TypeAnnotationKind::USize(lit) => Type::USize(*lit),
        TypeAnnotationKind::I8(lit) => Type::I8(*lit),
        TypeAnnotationKind::I16(lit) => Type::I16(*lit),
        TypeAnnotationKind::I32(lit) => Type::I32(*lit),
        TypeAnnotationKind::I64(lit) => Type::I64(*lit),
        TypeAnnotationKind::ISize(lit) => Type::ISize(*lit),
        TypeAnnotationKind::F32(lit) => Type::F32(lit.map(OrderedF32)),
        TypeAnnotationKind::F64(lit) => Type::F64(lit.map(OrderedF64)),
        TypeAnnotationKind::String(lit) => Type::Struct(StructKind::StringHeader(*lit)),
        TypeAnnotationKind::Identifier(id) => {
            check_type_identifier_annotation(ctx, id.clone())
        }
        TypeAnnotationKind::FnType {
            params,
            return_type,
        } => {
            let checked_params = check_params(ctx, params);
            let checked_return_type = check_type_annotation(ctx, return_type);

            Type::Fn(FnType {
                params: checked_params,
                return_type: Box::new(checked_return_type),
            })
        }
        TypeAnnotationKind::Union(variants) => {
            let mut checked_variants = Vec::new();

            for v in variants {
                checked_variants.push(check_type_annotation(ctx, v).kind);
            }

            Type::make_union(checked_variants)
        }
        TypeAnnotationKind::List(item_type) => {
            let checked_item_type = check_type_annotation(ctx, item_type);
            Type::List(Box::new(checked_item_type))
        }
        TypeAnnotationKind::Struct(items) => {
            let checked_field_types = check_params(ctx, items);
            Type::Struct(checked_field_types)
        }
    };

    SpannedType {
        kind,
        span: annotation.span.clone(),
    }
}
