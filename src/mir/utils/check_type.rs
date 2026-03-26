use crate::{
    ast::{
        decl::Param,
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        IdentifierNode,
    },
    compile::interner::TypeId,
    mir::{
        builders::{Builder, BuilderContext},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, CheckedParam, FnType},
            checked_type::{SpannedType, StructKind, Type},
            ordered_float::{OrderedF32, OrderedF64},
        },
        utils::layout::pack_struct,
    },
};

impl<'a, C: BuilderContext> Builder<'a, C> {
    pub fn check_params(&mut self, params: &[Param]) -> Vec<CheckedParam> {
        params
            .iter()
            .map(|p| CheckedParam {
                ty: self.check_type_annotation(&p.constraint),
                identifier: p.identifier.clone(),
            })
            .collect()
    }

    pub fn check_type_identifier_annotation(&mut self, id: IdentifierNode) -> TypeId {
        self.current_scope
            .lookup(id.name)
            .map(|entry| {
                match self.program.declarations.get(&entry).unwrap_or_else(|| {
                    panic!(
                        "INTERNAL COMPILER ERROR: Expected declarations to contain \
                     DeclarationId({}) key",
                        entry.0
                    )
                }) {
                    CheckedDeclaration::TypeAlias(decl) => decl.value.id,
                    CheckedDeclaration::Function(_) => {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::CannotUseFunctionDeclarationAsType,
                            span: id.span.clone(),
                        });

                        self.types.unknown()
                    }
                    CheckedDeclaration::Var(_) => {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::CannotUseVariableDeclarationAsType,
                            span: id.span.clone(),
                        });

                        self.types.unknown()
                    }
                }
            })
            .unwrap_or_else(|| {
                self.errors.push(SemanticError {
                    span: id.span.clone(),
                    kind: SemanticErrorKind::UndeclaredType(id),
                });

                self.types.unknown()
            })
    }

    pub fn check_type_annotation(&mut self, annotation: &TypeAnnotation) -> SpannedType {
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
            TypeAnnotationKind::String(lit) => {
                let inner = self
                    .types
                    .intern(&Type::Struct(StructKind::StringHeader(*lit)));

                self.types.ptr(inner)
            }
            TypeAnnotationKind::Identifier(id) => {
                self.check_type_identifier_annotation(id.clone())
            }
            TypeAnnotationKind::FnType {
                params,
                return_type,
            } => {
                let checked_params = self.check_params(params);
                let checked_return_type = self.check_type_annotation(return_type);

                self.types.intern(&Type::Fn(FnType::Indirect {
                    params: checked_params,
                    return_type: checked_return_type,
                }))
            }
            TypeAnnotationKind::Union(variants) => {
                let mut checked_variants = Vec::new();

                for v in variants {
                    checked_variants.push(self.check_type_annotation(v).id);
                }

                self.types.make_union(checked_variants)
            }
            TypeAnnotationKind::List(item_type) => {
                let checked_item_type = self.check_type_annotation(item_type);
                let inner = self
                    .types
                    .intern(&Type::Struct(StructKind::ListHeader(checked_item_type.id)));

                self.types.ptr(inner)
            }
            TypeAnnotationKind::Struct(items) => {
                let checked_field_types = self.check_params(items);
                let packed =
                    pack_struct(StructKind::UserDefined(checked_field_types), self.types);
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
