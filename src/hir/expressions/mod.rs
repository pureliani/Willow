pub mod access;
pub mod and;
pub mod binary;
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
    ast::expr::{Expr, ExprKind},
    hir::{
        builders::{Builder, InBlock},
        expressions::r#if::IfContext,
        instructions::{BinaryOpKind, InstrId},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_expr(&mut self, expr: Expr) -> InstrId {
        let span = expr.span.clone();
        let result = match expr.kind {
            ExprKind::Not { right } => self.build_not_expr(*right),
            ExprKind::Neg { right } => self.build_neg_expr(*right),
            ExprKind::Add { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Add)
            }
            ExprKind::Subtract { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Sub)
            }
            ExprKind::Multiply { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Mul)
            }
            ExprKind::Divide { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Div)
            }
            ExprKind::Modulo { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Rem)
            }
            ExprKind::LessThan { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Lt)
            }
            ExprKind::LessThanOrEqual { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Lte)
            }
            ExprKind::GreaterThan { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Gt)
            }
            ExprKind::GreaterThanOrEqual { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Gte)
            }
            ExprKind::Equal { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Eq)
            }
            ExprKind::NotEqual { left, right } => {
                self.build_binary(*left, *right, BinaryOpKind::Neq)
            }
            ExprKind::And { left, right } => self.build_and_expr(*left, *right),
            ExprKind::Or { left, right } => self.build_or_expr(*left, *right),
            ExprKind::BoolLiteral(value) => self.emit_bool(value),
            ExprKind::Number(number_kind) => self.emit_number(number_kind),
            ExprKind::String(string_node) => self.build_string_literal(string_node),
            ExprKind::Struct(fields) => self.build_struct_init_expr(span, fields, false),
            ExprKind::List(items) => self.build_list_literal_expr(span, items),
            ExprKind::Access { left, field } => self.build_access_expr(*left, field),
            ExprKind::StaticAccess { left, field } => {
                self.build_static_access_expr(*left, field)
            }
            ExprKind::If {
                branches,
                else_branch,
            } => self.build_if(branches, else_branch, IfContext::Expression),
            ExprKind::CodeBlock(block_contents) => {
                self.build_codeblock_expr(block_contents).0
            }
            ExprKind::Fn(fn_decl) => self.build_fn_expr(*fn_decl),
            ExprKind::FnCall { left, args } => self.build_fn_call_expr(*left, args, span),
            ExprKind::Identifier(identifier_node) => {
                self.build_identifier_expr(identifier_node)
            }
            ExprKind::TypeCast { left, target } => {
                self.build_typecast_expr(*left, target)
            }
            ExprKind::IsType { left, ty } => self.build_is_type_expr(*left, ty),
            ExprKind::Null => self.build_null_expr(span),
            ExprKind::TemplateString(parts) => self.build_template_expr(parts, span),
            ExprKind::GenericApply { left, type_args } => {
                self.build_generic_apply_expr(*left, type_args, span)
            }
        };

        self.check_expected(result, expr.span)
    }
}
