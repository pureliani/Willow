pub mod access;
pub mod and;
pub mod binary_op;
pub mod bool;
pub mod codeblock;
pub mod r#fn;
pub mod fn_call;
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
pub mod typecast;
pub mod unary_op;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        Span,
    },
    mir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        expressions::r#if::IfContext,
        types::checked_type::{SpannedType, Type},
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
            if value_type != &Type::Unknown && &et.kind != value_type {
                return self.report_error_and_get_poison(SemanticError {
                    kind: SemanticErrorKind::TypeMismatch {
                        expected: et.kind.clone(),
                        received: value_type.clone(),
                    },
                    span: value_span,
                });
            }
        }

        value
    }

    pub fn build_expr(
        &mut self,
        expr: Expr,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let span = expr.span.clone();
        let result = match expr.kind {
            ExprKind::Not { right } => self.build_not_expr(*right, expected_type),
            ExprKind::Neg { right } => self.build_neg_expr(*right, expected_type),
            ExprKind::Add { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_add, expected_type)
            }
            ExprKind::Subtract { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_sub, expected_type)
            }
            ExprKind::Multiply { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_mul, expected_type)
            }
            ExprKind::Divide { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_div, expected_type)
            }
            ExprKind::Modulo { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_rem, expected_type)
            }
            ExprKind::LessThan { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_lt, expected_type)
            }
            ExprKind::LessThanOrEqual { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_lte, expected_type)
            }
            ExprKind::GreaterThan { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_gt, expected_type)
            }
            ExprKind::GreaterThanOrEqual { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_gte, expected_type)
            }
            ExprKind::Equal { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_eq, expected_type)
            }
            ExprKind::NotEqual { left, right } => {
                self.build_binary_op(*left, *right, Self::emit_neq, expected_type)
            }
            ExprKind::And { left, right } => {
                self.build_and_expr(*left, *right, expected_type)
            }
            ExprKind::Or { left, right } => {
                self.build_or_expr(*left, *right, expected_type)
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
            ExprKind::Struct(fields) => {
                self.build_struct_init_expr(span, fields, expected_type)
            }
            ExprKind::List(items) => {
                self.build_list_literal_expr(span, items, expected_type)
            }
            ExprKind::Access { left, field } => {
                self.build_access_expr(*left, field, expected_type)
            }
            ExprKind::StaticAccess { left, field } => {
                self.build_static_access_expr(*left, field, expected_type)
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                self.build_if(branches, else_branch, IfContext::Expression, expected_type)
            }
            ExprKind::CodeBlock(block_contents) => {
                self.build_codeblock_expr(block_contents, expected_type, false).0
            }
            ExprKind::UnsafeBlock(block_contents) => {
                self.build_codeblock_expr(block_contents, expected_type, true).0
            }
            ExprKind::Fn(fn_decl) => self.build_fn_expr(*fn_decl, expected_type),
            ExprKind::FnCall { left, args } => {
                self.build_fn_call_expr(*left, args, span, expected_type)
            }
            ExprKind::Identifier(identifier_node) => {
                self.build_identifier_expr(identifier_node, expected_type)
            }
            ExprKind::TypeCast { left, target } => {
                self.build_typecast_expr(*left, target, expected_type)
            }
            ExprKind::IsType { left, ty } => {
                self.build_is_type_expr(*left, ty, expected_type)
            }
            ExprKind::Null => self.build_null_expr(span, expected_type),
        };

        self.check_expected(result, expr.span, expected_type)
    }
}
