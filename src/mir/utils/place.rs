use std::collections::HashMap;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        DeclarationId, IdentifierNode, Span,
    },
    compile::interner::StringId,
    mir::{
        builders::{BasicBlockId, Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, FnType},
            checked_type::{StructKind, Type},
        },
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Local(DeclarationId),
    Field(Box<Place>, StringId),
    Temporary(ValueId),
}

impl Place {
    pub fn root(&self) -> Option<DeclarationId> {
        match self {
            Place::Local(id) => Some(*id),
            Place::Field(base, _) => base.root(),
            Place::Temporary(_) => None,
        }
    }

    /// For `x.a.b`, returns `[a, b]`
    pub fn path(&self) -> Vec<StringId> {
        let mut path = Vec::new();
        let mut current = self;
        while let Place::Field(base, field) = current {
            path.push(*field);
            current = base;
        }
        path.reverse();
        path
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Place::Local(_))
    }

    pub fn canonicalize(&self, aliases: &HashMap<DeclarationId, Place>) -> Place {
        match self {
            Place::Local(decl_id) => {
                if let Some(aliased) = aliases.get(decl_id) {
                    aliased.canonicalize(aliases)
                } else {
                    self.clone()
                }
            }
            Place::Field(base, field) => {
                Place::Field(Box::new(base.canonicalize(aliases)), *field)
            }
            Place::Temporary(_) => self.clone(),
        }
    }
}

impl<'a> Builder<'a, InBlock> {
    pub fn resolve_place(&mut self, expr: Expr) -> Result<Place, SemanticError> {
        let span = expr.span.clone();

        match expr.kind {
            ExprKind::Identifier(ident) => match self.current_scope.lookup(ident.name) {
                Some(decl_id) => Ok(Place::Local(decl_id)),
                None => Err(SemanticError {
                    kind: SemanticErrorKind::UndeclaredIdentifier(ident),
                    span,
                }),
            },
            ExprKind::Access { left, field } => {
                let base = self.resolve_place(*left)?;

                let current_val = self.read_place(&base, field.span.clone());
                let current_type = self.get_value_type(current_val).clone();

                self.validate_field_access(&current_type, &field)?;

                Ok(Place::Field(Box::new(base), field.name))
            }
            _ => {
                let val_id = self.build_expr(expr, None);
                Ok(Place::Temporary(val_id))
            }
        }
    }

    /// Checks that the given field is accessible on this type from user code.
    /// Internal struct fields (union id/value, list headers, string headers)
    /// are not user-accessible.
    fn validate_field_access(
        &self,
        base_type: &Type,
        field: &IdentifierNode,
    ) -> Result<(), SemanticError> {
        let struct_kind = match base_type {
            Type::Pointer(inner) => match &**inner {
                Type::Struct(s) => s,
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };

        match struct_kind {
            StructKind::UserDefined(_) => Ok(()),
            _ => Err(SemanticError {
                span: field.span.clone(),
                kind: SemanticErrorKind::AccessToUndefinedField(field.clone()),
            }),
        }
    }

    pub fn read_place(&mut self, place: &Place, span: Span) -> ValueId {
        let canonical = place.canonicalize(self.aliases);
        self.read_place_from_block(self.context.block_id, &canonical, span)
    }

    pub fn read_place_from_block(
        &mut self,
        block_id: BasicBlockId,
        place: &Place,
        span: Span,
    ) -> ValueId {
        if let Place::Temporary(val_id) = place {
            return *val_id;
        }

        if let Some(block_defs) = self.current_defs.get(&block_id) {
            if let Some(val) = block_defs.get(place) {
                return *val;
            }
        }

        let predecessors: Vec<BasicBlockId> =
            self.get_bb(block_id).predecessors.iter().cloned().collect();

        let val_id = if !self.get_bb(block_id).sealed {
            let val_id = self.new_value_id(Type::Unknown);
            self.incomplete_phis.entry(block_id).or_default().push((
                val_id,
                place.clone(),
                span,
            ));
            val_id
        } else if predecessors.len() == 1 {
            self.read_place_from_block(predecessors[0], place, span)
        } else if predecessors.is_empty() {
            panic!(
                "INTERNAL COMPILER ERROR: Tried to read place in a basic block which \
                 neither defined the place nor had the predecessors"
            );
        } else {
            let val_id = self.new_value_id(Type::Unknown);

            self.current_defs
                .entry(block_id)
                .or_default()
                .insert(place.clone(), val_id);

            self.resolve_phi(block_id, val_id, place, span);
            val_id
        };

        self.current_defs
            .entry(block_id)
            .or_default()
            .insert(place.clone(), val_id);

        val_id
    }

    pub fn remap_place(&mut self, place: &Place, value: ValueId) {
        if let Place::Temporary(_) = place {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot remap (SSA update) a temporary place, \
                 temporaries are R-Values"
            );
        }

        let canonical = place.canonicalize(self.aliases);
        self.current_defs
            .entry(self.context.block_id)
            .or_default()
            .insert(canonical, value);
    }

    pub fn type_of_place(&self, place: &Place) -> Type {
        match place {
            Place::Local(decl_id) => {
                let decl = self.program.declarations.get(decl_id).unwrap();
                match decl {
                    CheckedDeclaration::Var(v) => {
                        self.get_value_type(v.stack_ptr).unwrap_ptr().clone()
                    }
                    CheckedDeclaration::Function(f) => Type::Fn(FnType::Direct(f.id)),
                    _ => panic!(),
                }
            }
            Place::Field(base, field) => {
                let base_ty = self.type_of_place(base);
                self.type_of_field(&base_ty, *field).expect(
                    "INTERNAL COMPILER ERROR: Expected Place::Field to have a type",
                )
            }
            Place::Temporary(val_id) => self
                .program
                .value_types
                .get(val_id)
                .expect(
                    "INTERNAL COMPILER ERROR: Expected Place::Temporary to have a type",
                )
                .clone(),
        }
    }

    fn type_of_field(&self, base_ty: &Type, field: StringId) -> Option<Type> {
        use Type::*;

        let base_inner = match base_ty {
            Pointer(inner) => inner,
            _ => return None,
        };

        let struct_kind = match base_inner.as_ref() {
            Struct(s) => s,
            _ => return None,
        };

        struct_kind.get_field(&field).map(|(_, ty)| ty)
    }
}
