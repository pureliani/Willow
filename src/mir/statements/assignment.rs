use crate::{
    ast::{
        expr::{Expr, ExprKind},
        Span,
    },
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::CheckedDeclaration,
            checked_type::{SpannedType, Type},
        },
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn write_expr(
        &mut self,
        target: &Expr,
        value: ValueId,
        span: Span,
    ) -> Result<(), SemanticError> {
        match &target.kind {
            ExprKind::Identifier(ident) => {
                let decl_id = self.current_scope.lookup(ident.name).ok_or_else(|| {
                    SemanticError {
                        kind: SemanticErrorKind::UndeclaredIdentifier(ident.clone()),
                        span: span.clone(),
                    }
                })?;

                self.write_variable(decl_id, self.context.block_id, value);
                Ok(())
            }
            ExprKind::Access { left, field } => {
                let base_val = self.build_expr(*left.clone(), None);
                let new_base_val =
                    self.emit_update_struct_field(base_val, field.clone(), value);
                self.write_expr(left, new_base_val, span)
            }
            _ => Err(SemanticError {
                kind: SemanticErrorKind::InvalidLValue,
                span,
            }),
        }
    }

    pub fn build_assignment_stmt(&mut self, target: Expr, value: Expr) {
        let target_span = target.span.clone();

        let constraint = match self.get_constraint_type(&target) {
            Some(c) => c.clone(),
            None => {
                self.errors.push(SemanticError {
                    kind: SemanticErrorKind::InvalidLValue,
                    span: target_span,
                });
                return;
            }
        };

        let value_id = self.build_expr(value, Some(&constraint));

        if let Err(e) = self.write_expr(&target, value_id, target_span) {
            self.errors.push(e);
        }
    }

    pub fn get_constraint_type(&self, expr: &Expr) -> Option<&SpannedType> {
        match &expr.kind {
            ExprKind::Identifier(ident) => {
                let decl_id = self.current_scope.lookup(ident.name)?;
                match self.program.declarations.get(&decl_id)? {
                    CheckedDeclaration::Var(v) => {
                        let constraint = self.get_value_type(v.stack_ptr);
                        if let Type::Pointer(to) = constraint {
                            Some(&SpannedType {
                                kind: **to,
                                span: v.constraint_span,
                            })
                        } else {
                            panic!("INTERNAL COMPILER ERROR: Variables must be stack allocated")
                        }
                    }
                    _ => None,
                }
            }
            ExprKind::Access { left, field } => {
                let parent_constraint = self.get_constraint_type(left)?;
                parent_constraint
                    .kind
                    .get_field(&field.name)
                    .map(|(_, ty)| ty)
            }
            _ => None,
        }
    }
}
