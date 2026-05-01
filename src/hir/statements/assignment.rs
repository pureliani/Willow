use crate::{
    ast::decl::Declaration,
    ast::expr::{Expr, ExprKind},
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        instructions::Place,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_place(&mut self, expr: Expr) -> Result<Place, SemanticError> {
        match expr.kind {
            ExprKind::Identifier(ident) => {
                if let Some(decl_id) = self.current_scope.lookup(ident.name) {
                    let decl = self.program.declarations.get(&decl_id).unwrap();
                    if matches!(decl, Declaration::Var(_)) {
                        Ok(Place::Var(decl_id))
                    } else {
                        Err(SemanticError {
                            kind: SemanticErrorKind::InvalidLValue,
                            span: ident.span,
                        })
                    }
                } else {
                    Err(SemanticError {
                        kind: SemanticErrorKind::UndeclaredIdentifier(ident.clone()),
                        span: ident.span,
                    })
                }
            }
            ExprKind::Access { left, field } => {
                let base_place = self.build_place(*left)?;
                Ok(Place::Field(Box::new(base_place), field))
            }
            ExprKind::Index { left, index } => {
                let base_place = self.build_place(*left)?;
                let index_id = self.build_expr(*index);
                Ok(Place::Index(Box::new(base_place), index_id))
            }
            _ => {
                let instr_id = self.build_expr(expr);
                Ok(Place::Expr(instr_id))
            }
        }
    }

    pub fn build_assignment_stmt(&mut self, target: Expr, value: Expr) {
        let target_span = target.span.clone();

        let place = match self.build_place(target) {
            Ok(p) => p,
            Err(e) => {
                self.errors.push(e);
                return;
            }
        };

        let rhs_id = self.build_expr(value);

        match place {
            Place::Var(decl_id) => {
                self.write_variable(self.context.block_id, decl_id, rhs_id);
            }
            _ => {
                self.emit_write_place(place, rhs_id, target_span);
            }
        }
    }
}
