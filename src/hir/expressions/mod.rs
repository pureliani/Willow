pub mod access;
pub mod and;
pub mod binary_op;
pub mod bool;
pub mod codeblock;
pub mod r#fn;
pub mod fn_call;
pub mod generic_apply;
pub mod identifier;
pub mod r#if;
pub mod is_type;
pub mod list_literal;
pub mod null;
pub mod number;
pub mod or;
pub mod static_access;
pub mod string;
pub mod struct_init;
pub mod template;
pub mod typecast;
pub mod unary_op;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        Span,
    },
    compile::interner::GenericSubstitutions,
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        expressions::r#if::IfContext,
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn check_expected(
        &mut self,
        value: ValueId,
        value_span: Span,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let value_type = self.get_value_type(value);

        if let Some(et) = expected_type {
            if value_type != self.types.unknown() && et.id != value_type {
                match self.compute_type_adjustment(value_type, et.id, false) {
                    Ok(adj) => {
                        return self.apply_adjustment(value, adj, et.id, value_span);
                    }
                    Err(_) => {
                        return self.report_error_and_get_poison(SemanticError {
                            kind: SemanticErrorKind::TypeMismatch {
                                expected: et.id,
                                received: value_type,
                            },
                            span: value_span,
                        });
                    }
                }
            }
        }

        value
    }

    pub fn build_expr(
        &mut self,
        expr: Expr,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let span = expr.span.clone();
        let result = match expr.kind {
            ExprKind::Not { right } => {
                self.build_not_expr(*right, expected_type, substitutions)
            }
            ExprKind::Neg { right } => {
                self.build_neg_expr(*right, expected_type, substitutions)
            }
            ExprKind::Add { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::add,
                expected_type,
                substitutions,
            ),
            ExprKind::Subtract { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::sub,
                expected_type,
                substitutions,
            ),
            ExprKind::Multiply { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::mul,
                expected_type,
                substitutions,
            ),
            ExprKind::Divide { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::div,
                expected_type,
                substitutions,
            ),
            ExprKind::Modulo { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::rem,
                expected_type,
                substitutions,
            ),
            ExprKind::LessThan { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::lt,
                expected_type,
                substitutions,
            ),
            ExprKind::LessThanOrEqual { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::lte,
                expected_type,
                substitutions,
            ),
            ExprKind::GreaterThan { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::gt,
                expected_type,
                substitutions,
            ),
            ExprKind::GreaterThanOrEqual { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::gte,
                expected_type,
                substitutions,
            ),
            ExprKind::Equal { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::eq,
                expected_type,
                substitutions,
            ),
            ExprKind::NotEqual { left, right } => self.build_binary_op(
                *left,
                *right,
                Self::neq,
                expected_type,
                substitutions,
            ),
            ExprKind::And { left, right } => {
                self.build_and_expr(*left, *right, expected_type, substitutions)
            }
            ExprKind::Or { left, right } => {
                self.build_or_expr(*left, *right, expected_type, substitutions)
            }
            ExprKind::BoolLiteral(value) => {
                self.build_bool_expr(span, value, expected_type)
            }
            ExprKind::Number(number_kind) => {
                self.build_number_expr(span, number_kind, expected_type)
            }
            ExprKind::String(string_node) => {
                self.build_string_literal(string_node, expected_type)
            }
            ExprKind::Struct(fields) => self.build_struct_init_expr(
                span,
                fields,
                expected_type,
                false,
                substitutions,
            ),
            ExprKind::List(items) => {
                self.build_list_literal_expr(span, items, expected_type, substitutions)
            }
            ExprKind::Access { left, field } => {
                self.build_access_expr(*left, field, expected_type, substitutions)
            }
            ExprKind::StaticAccess { left, field } => {
                self.build_static_access_expr(*left, field, expected_type, substitutions)
            }
            ExprKind::If {
                branches,
                else_branch,
            } => self.build_if(
                branches,
                else_branch,
                IfContext::Expression,
                expected_type,
                substitutions,
            ),
            ExprKind::CodeBlock(block_contents) => {
                self.build_codeblock_expr(block_contents, expected_type, substitutions)
                    .0
            }
            ExprKind::Fn(fn_decl) => {
                self.build_fn_expr(*fn_decl, expected_type, substitutions)
            }
            ExprKind::FnCall { left, args } => {
                self.build_fn_call_expr(*left, args, span, expected_type, substitutions)
            }
            ExprKind::Identifier(identifier_node) => {
                self.build_identifier_expr(identifier_node, expected_type, substitutions)
            }
            ExprKind::TypeCast { left, target } => {
                self.build_typecast_expr(*left, target, expected_type, substitutions)
            }
            ExprKind::IsType { left, ty } => {
                self.build_is_type_expr(*left, ty, expected_type, substitutions)
            }
            ExprKind::Null => self.build_null_expr(span, expected_type),
            ExprKind::TemplateString(parts) => {
                self.build_template_expr(parts, span, expected_type)
            }
            ExprKind::GenericApply { left, type_args } => self.build_generic_apply_expr(
                *left,
                type_args,
                span,
                expected_type,
                substitutions,
            ),
        };

        self.check_expected(result, expr.span, expected_type)
    }
}
