// src/mir/utils/place.rs
use std::collections::HashMap;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        DeclarationId, IdentifierNode, Span,
    },
    compile::interner::{StringId, TypeId},
    mir::{
        builders::{BasicBlockId, Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, FnType},
            checked_type::{StructKind, Type},
        },
        utils::facts::{narrowed_type::NarrowedTypeFact, FactSet},
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Var(DeclarationId),
    Deref(Box<Place>),
    Field(Box<Place>, StringId),
    Temporary(ValueId),
}

impl Place {
    pub fn root(&self) -> Option<DeclarationId> {
        match self {
            Place::Var(id) => Some(*id),
            Place::Deref(base) => base.root(),
            Place::Field(base, _) => base.root(),
            Place::Temporary(_) => None,
        }
    }

    pub fn path(&self) -> Vec<StringId> {
        let mut path = Vec::new();
        let mut current = self;

        // Traverse up, skipping Derefs, to build the field path
        loop {
            match current {
                Place::Field(base, field) => {
                    path.push(*field);
                    current = base;
                }
                Place::Deref(base) => {
                    current = base;
                }
                _ => break,
            }
        }
        path.reverse();
        path
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Place::Var(_))
    }

    pub fn canonicalize(&self, aliases: &HashMap<DeclarationId, Place>) -> Place {
        match self {
            Place::Var(decl_id) => {
                if let Some(aliased) = aliases.get(decl_id) {
                    aliased.canonicalize(aliases)
                } else {
                    self.clone()
                }
            }
            Place::Deref(base) => Place::Deref(Box::new(base.canonicalize(aliases))),
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
                Some(decl_id) => {
                    let decl = self.program.declarations.get(&decl_id).unwrap();
                    match decl {
                        CheckedDeclaration::Var(_) => Ok(Place::Var(decl_id)),
                        _ => Err(SemanticError {
                            kind: SemanticErrorKind::InvalidLValue, // Functions/Types are not L-values!
                            span,
                        }),
                    }
                }
                None => Err(SemanticError {
                    kind: SemanticErrorKind::UndeclaredIdentifier(ident),
                    span,
                }),
            },
            ExprKind::Access { left, field } => {
                let base_place = self.resolve_place(*left)?;
                let base_ty = self.type_of_place(&base_place);

                // If the base is a pointer (e.g. ptr<Struct>), we MUST dereference it
                // to access its fields.
                let derefed_place = if self.types.is_pointer(base_ty) {
                    Place::Deref(Box::new(base_place))
                } else {
                    base_place
                };

                // Validate field access against the underlying Struct type
                let struct_ty = self.type_of_place(&derefed_place);
                self.validate_field_access(struct_ty, &field)?;

                Ok(Place::Field(Box::new(derefed_place), field.name))
            }
            _ => {
                let val_id = self.build_expr(expr, None);
                Ok(Place::Temporary(val_id))
            }
        }
    }

    fn validate_field_access(
        &self,
        struct_ty: TypeId,
        field: &IdentifierNode,
    ) -> Result<(), SemanticError> {
        let struct_kind = match self.types.resolve(struct_ty) {
            Type::Struct(s) => s,
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

    pub fn get_place_ptr(&mut self, place: &Place, span: Span) -> ValueId {
        match place {
            Place::Var(decl_id) => {
                let decl = self.program.declarations.get(decl_id).unwrap();
                match decl {
                    CheckedDeclaration::Var(v) => v.stack_ptr,
                    _ => {
                        panic!("INTERNAL COMPILER ERROR: Place::Var is not a variable")
                    }
                }
            }
            Place::Deref(base) => {
                // The memory address of `*base` is simply the value of `base`!
                self.read_place(base, span)
            }
            Place::Field(base, field) => {
                let base_ptr = self.get_place_ptr(base, span.clone());
                self.get_field_ptr(base_ptr, *field)
            }
            Place::Temporary(val_id) => {
                // A temporary is an R-value. It has no memory address inherently.
                // If we are asked for its address (e.g., to access a field like `get_struct().a`),
                // we MUST spill it to the stack to give it a physical memory address.
                let ty = self.get_value_type(*val_id);
                let ptr = self.emit_stack_alloc(ty, 1);
                self.emit_store(ptr, *val_id);
                ptr
            }
        }
    }

    pub fn read_place(&mut self, place: &Place, span: Span) -> ValueId {
        if let Place::Temporary(val_id) = place {
            return *val_id;
        }

        let ptr = self.get_place_ptr(place, span.clone());
        let val = self.emit_load(ptr);

        let facts = self.read_fact_from_block(self.context.block_id, place);
        if let Some(narrowed) = facts.get::<NarrowedTypeFact>() {
            let current_ty = self.get_value_type(val);

            if narrowed.variants.len() == 1 {
                let target_ty = *narrowed.variants.iter().next().unwrap();
                if current_ty != target_ty {
                    return self.unwrap_from_union(val, target_ty);
                }
            } else if narrowed.variants.len()
                < self
                    .types
                    .get_union_variants(current_ty)
                    .map(|v| v.len())
                    .unwrap_or(0)
            {
                let target_ty = self.types.make_union(narrowed.variants.iter().cloned());
                return self.coerce_to_union(val, target_ty);
            }
        }

        val
    }

    pub fn write_place(&mut self, place: &Place, value: ValueId, span: Span) {
        if let Place::Temporary(_) = place {
            panic!("INTERNAL COMPILER ERROR: Cannot write to a temporary r-value");
        }

        let ptr = self.get_place_ptr(place, span);
        self.emit_store(ptr, value);

        self.write_fact(self.context.block_id, place, FactSet::new());
    }

    pub fn type_of_place(&self, place: &Place) -> TypeId {
        match place {
            Place::Var(decl_id) => {
                let decl = self.program.declarations.get(decl_id).unwrap();
                match decl {
                    CheckedDeclaration::Var(v) => {
                        let ptr_ty = self.get_value_type(v.stack_ptr);
                        self.types.unwrap_ptr(ptr_ty)
                    }
                    CheckedDeclaration::Function(f) => {
                        self.types.intern(&Type::Fn(FnType::Direct(f.id)))
                    }
                    _ => panic!("Invalid local place"),
                }
            }
            Place::Deref(base) => {
                let base_ty = self.type_of_place(base);
                self.types.unwrap_ptr(base_ty)
            }
            Place::Field(base, field) => {
                let base_ty = self.type_of_place(base);
                self.type_of_field(base_ty, *field).expect(
                    "INTERNAL COMPILER ERROR: Expected Place::Field to have a type",
                )
            }
            Place::Temporary(val_id) => *self.program.value_types.get(val_id).expect(
                "INTERNAL COMPILER ERROR: Expected Place::Temporary to have a type",
            ),
        }
    }

    fn type_of_field(&self, struct_ty: TypeId, field: StringId) -> Option<TypeId> {
        // Because of Deref, struct_ty is now guaranteed to be the Struct itself, not a pointer.
        let struct_kind = match self.types.resolve(struct_ty) {
            Type::Struct(s) => s,
            _ => return None,
        };

        struct_kind.get_field(self.types, &field).map(|f| f.ty)
    }

    pub fn read_fact_from_block(
        &mut self,
        block_id: BasicBlockId,
        place: &Place,
    ) -> FactSet {
        if let Some(block_facts) = self.current_facts.get(&block_id) {
            if let Some(fact_set) = block_facts.get(place) {
                return fact_set.clone();
            }
        }

        let predecessors: Vec<BasicBlockId> =
            self.get_bb(block_id).predecessors.iter().cloned().collect();

        let fact_set = if !self.get_bb(block_id).sealed {
            self.incomplete_fact_merges
                .entry(block_id)
                .or_default()
                .push(place.clone());
            FactSet::new()
        } else if predecessors.len() == 1 {
            self.read_fact_from_block(predecessors[0], place)
        } else if predecessors.is_empty() {
            FactSet::new()
        } else {
            let mut merged_facts = self.read_fact_from_block(predecessors[0], place);
            for &pred in predecessors.iter().skip(1) {
                let pred_facts = self.read_fact_from_block(pred, place);
                merged_facts = merged_facts.merge(&pred_facts);
            }
            merged_facts
        };

        self.current_facts
            .entry(block_id)
            .or_default()
            .insert(place.clone(), fact_set.clone());
        fact_set
    }

    pub fn write_fact(
        &mut self,
        block_id: BasicBlockId,
        place: &Place,
        fact_set: FactSet,
    ) {
        self.current_facts
            .entry(block_id)
            .or_default()
            .insert(place.clone(), fact_set);
    }
}
