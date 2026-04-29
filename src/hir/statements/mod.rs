pub mod assignment;
pub mod from;
pub mod r#return;
pub mod type_alias_decl;
pub mod var_decl;
pub mod r#while;

use crate::{
    ast::{
        expr::ExprKind,
        stmt::{Stmt, StmtKind},
    },
    compile::interner::GenericSubstitutions,
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        expressions::r#if::IfContext,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_statements(
        &mut self,
        statements: Vec<Stmt>,
        substitutions: &GenericSubstitutions,
    ) {
        for statement in statements {
            if self.bb().terminator.is_some() {
                self.errors.push(SemanticError {
                    kind: SemanticErrorKind::UnreachableCode,
                    span: statement.span,
                });
                break;
            }

            match statement.kind {
                StmtKind::Expression(expr) => {
                    if let ExprKind::If {
                        branches,
                        else_branch,
                    } = expr.kind
                    {
                        self.build_if(
                            branches,
                            else_branch,
                            IfContext::Statement,
                            None,
                            substitutions,
                        );
                    } else {
                        self.build_expr(expr, None, substitutions);
                    }
                }
                StmtKind::TypeAliasDecl(decl) => self.as_module().build_type_alias_decl(
                    decl,
                    statement.span,
                    substitutions,
                ),
                StmtKind::VarDecl(var_decl) => {
                    self.build_var_decl(var_decl, substitutions)
                }
                StmtKind::Return { value } => {
                    self.build_return_stmt(value, statement.span, substitutions)
                }
                StmtKind::Assignment { target, value } => {
                    self.build_assignment_stmt(target, value, substitutions)
                }
                StmtKind::From { path, items } => {
                    self.as_module()
                        .build_from_stmt(path, items, statement.span);
                }
                StmtKind::While { condition, body } => {
                    self.build_while_stmt(*condition, body, substitutions)
                }
                StmtKind::Break => {
                    if let Some(targets) = self.current_scope.within_loop_body() {
                        self.emit_jmp(targets.on_break);
                    } else {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::BreakKeywordOutsideLoop,
                            span: statement.span,
                        })
                    }
                }
                StmtKind::Continue => {
                    if let Some(targets) = self.current_scope.within_loop_body() {
                        self.emit_jmp(targets.on_continue);
                    } else {
                        self.errors.push(SemanticError {
                            kind: SemanticErrorKind::ContinueKeywordOutsideLoop,
                            span: statement.span,
                        })
                    }
                }
            }
        }
    }
}
