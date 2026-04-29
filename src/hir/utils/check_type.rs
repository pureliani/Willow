use std::collections::HashMap;

use crate::{
    ast::{
        decl::Param,
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        IdentifierNode, Span, SymbolId,
    },
    compile::interner::{StringId, TypeId},
    globals::STRING_INTERNER,
    hir::{
        builders::{Builder, BuilderContext},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{
                CheckedDeclaration, CheckedParam, FnType, GenericDeclaration,
            },
            checked_type::{SpannedType, StructKind, Type},
            ordered_float::{OrderedF32, OrderedF64},
        },
        utils::{layout::pack_struct, scope::ScopeKind},
    },
};

impl<'a, C: BuilderContext> Builder<'a, C> {
    pub fn satisfies_extends_bound(&self, source: TypeId, target: TypeId) -> bool {
        let source = self.types.unwrap_generic_bound(source);
        let target = self.types.unwrap_generic_bound(target);

        if source == target {
            return true;
        }

        if let Some(src_variants) = self.types.get_union_variants(source) {
            for &variant in &src_variants {
                if !self.satisfies_extends_bound(variant, target) {
                    return false;
                }
            }
            return true;
        }

        if let Some(tgt_variants) = self.types.get_union_variants(target) {
            for &variant in &tgt_variants {
                if self.satisfies_extends_bound(source, variant) {
                    return true;
                }
            }
            return false;
        }

        let src_res = self.types.resolve(source);
        let tgt_res = self.types.resolve(target);

        if let Type::Literal(lit) = src_res {
            let widened = self.types.widen_literal(lit);
            if widened != source && self.satisfies_extends_bound(widened, target) {
                return true;
            }
        }

        if let Type::GenericParam {
            extends: Some(c), ..
        } = src_res
        {
            if self.satisfies_extends_bound(c, target) {
                return true;
            }
        }
        if let Type::GenericParam {
            extends: Some(c), ..
        } = tgt_res
        {
            if self.satisfies_extends_bound(source, c) {
                return true;
            }
        }

        if let (
            Type::Struct(StructKind::UserDefined(src_fields)),
            Type::Struct(StructKind::UserDefined(tgt_fields)),
        ) = (&src_res, &tgt_res)
        {
            for tgt_f in tgt_fields {
                let src_f = src_fields
                    .iter()
                    .find(|f| f.identifier.name == tgt_f.identifier.name);
                if let Some(src_f) = src_f {
                    if !self.satisfies_extends_bound(src_f.ty.id, tgt_f.ty.id) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            return true;
        }

        if let (
            Type::Struct(StructKind::ListHeader(src_inner)),
            Type::Struct(StructKind::ListHeader(tgt_inner)),
        ) = (&src_res, &tgt_res)
        {
            return self.satisfies_extends_bound(*src_inner, *tgt_inner);
        }

        if let (Type::Pointer(src_inner), Type::Pointer(tgt_inner)) = (&src_res, &tgt_res)
        {
            return self.satisfies_extends_bound(*src_inner, *tgt_inner);
        }

        if let (Type::IndirectFn(src_fn), Type::IndirectFn(tgt_fn)) = (&src_res, &tgt_res)
        {
            if src_fn.params.len() != tgt_fn.params.len() {
                return false;
            }

            for (s_param, t_param) in src_fn.params.iter().zip(tgt_fn.params.iter()) {
                if !self.satisfies_extends_bound(t_param.ty.id, s_param.ty.id) {
                    return false;
                }
            }

            if !self.satisfies_extends_bound(src_fn.return_type.id, tgt_fn.return_type.id)
            {
                return false;
            }

            return true;
        }

        false
    }

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

    pub fn check_type_identifier_annotation(
        &mut self,
        id: IdentifierNode,
        substitutions: &HashMap<StringId, TypeId>,
    ) -> TypeId {
        match self.current_scope.lookup(id.name) {
            Some(SymbolId::Concrete(decl_id)) => {
                match self.program.declarations.get(&decl_id).unwrap() {
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
            Some(SymbolId::GenericParameter(name)) => {
                if let Some(&ty) = substitutions.get(&name) {
                    ty
                } else {
                    panic!("INTERNAL COMPILER ERROR: Generic parameter {} found in scope but missing from substitutions map", STRING_INTERNER.resolve(name));
                }
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

    fn check_generic_apply_type_annotation(
        &mut self,
        left: &TypeAnnotation,
        args: &[TypeAnnotation],
        span: &Span,
        substitutions: &HashMap<StringId, TypeId>,
    ) -> TypeId {
        let id = match &left.kind {
            TypeAnnotationKind::Identifier(id) => id,
            _ => {
                self.errors.push(SemanticError {
                    span: left.span.clone(),
                    kind: SemanticErrorKind::CannotApplyTypeArguments,
                });
                return self.types.unknown();
            }
        };

        let symbol = match self.current_scope.lookup(id.name) {
            Some(sym) => sym,
            None => {
                self.errors.push(SemanticError {
                    span: id.span.clone(),
                    kind: SemanticErrorKind::UndeclaredType(id.clone()),
                });
                return self.types.unknown();
            }
        };

        let gen_id = match symbol {
            SymbolId::Generic(gen_id) => gen_id,
            SymbolId::Concrete(_) | SymbolId::GenericParameter(_) => {
                self.errors.push(SemanticError {
                    span: left.span.clone(),
                    kind: SemanticErrorKind::CannotApplyTypeArguments,
                });
                return self.types.unknown();
            }
        };

        let generic_decl = self
            .program
            .generic_declarations
            .get(&gen_id)
            .unwrap()
            .clone();

        let (decl, decl_scope, has_errors) = match generic_decl {
            GenericDeclaration::TypeAlias {
                decl,
                decl_scope,
                has_errors,
            } => (decl, decl_scope, has_errors),
            GenericDeclaration::Function { .. } => {
                self.errors.push(SemanticError {
                    span: id.span.clone(),
                    kind: SemanticErrorKind::IdentifierIsNotAType(id.clone()),
                });
                return self.types.unknown();
            }
        };

        if has_errors {
            return self.types.unknown();
        }

        if decl.generic_params.len() != args.len() {
            self.errors.push(SemanticError {
                span: span.clone(),
                kind: SemanticErrorKind::GenericArgumentCountMismatch {
                    expected: decl.generic_params.len(),
                    received: args.len(),
                },
            });
            return self.types.unknown();
        }

        let mut evaluated_args = Vec::with_capacity(args.len());
        for arg in args {
            evaluated_args.push(self.check_type_annotation(arg, substitutions).id);
        }

        let mut inner_substitutions = HashMap::new();
        let mut bounds_failed = false;

        for (param, &arg_ty) in decl.generic_params.iter().zip(evaluated_args.iter()) {
            inner_substitutions.insert(param.identifier.name, arg_ty);

            if let Some(bound_ast) = &param.extends {
                let bound_ty = self
                    .check_type_annotation(bound_ast, &inner_substitutions)
                    .id;

                if !self.satisfies_extends_bound(arg_ty, bound_ty) {
                    self.errors.push(SemanticError {
                        span: span.clone(),
                        kind: SemanticErrorKind::TypeMismatch {
                            expected: bound_ty,
                            received: arg_ty,
                        },
                    });
                    bounds_failed = true;
                }
            }
        }

        if bounds_failed {
            return self.types.unknown();
        }

        let caller_scope = self.current_scope.clone();
        self.current_scope = decl_scope.enter(ScopeKind::GenericParams, span.start);
        for param in &decl.generic_params {
            self.current_scope.map_name_to_symbol(
                param.identifier.name,
                SymbolId::GenericParameter(param.identifier.name),
            );
        }

        let result_id = self
            .check_type_annotation(&decl.value, &inner_substitutions)
            .id;

        self.current_scope = caller_scope;

        result_id
    }

    pub fn check_type_annotation(
        &mut self,
        annotation: &TypeAnnotation,
        substitutions: &HashMap<StringId, TypeId>,
    ) -> SpannedType {
        let id = match &annotation.kind {
            TypeAnnotationKind::Never => self.types.never(),
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
                self.check_type_identifier_annotation(id.clone(), substitutions)
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
            TypeAnnotationKind::GenericApply { left, args } => self
                .check_generic_apply_type_annotation(
                    left,
                    args,
                    &annotation.span,
                    substitutions,
                ),

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
