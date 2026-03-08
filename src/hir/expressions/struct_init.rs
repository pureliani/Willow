use std::collections::HashSet;

use crate::{
    ast::{expr::Expr, IdentifierNode, Span},
    compile::interner::StringId,
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::checked_type::SpannedType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_struct_init_expr(
        &mut self,
        span: Span,
        fields: Vec<(IdentifierNode, Expr)>,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let mut field_values: Vec<(IdentifierNode, ValueId)> =
            Vec::with_capacity(fields.len());
        let mut seen_names: HashSet<StringId> = HashSet::new();

        for (field_name, field_expr) in fields {
            if !seen_names.insert(field_name.name) {
                self.errors.push(SemanticError {
                    kind: SemanticErrorKind::DuplicateStructFieldInitializer(
                        field_name.clone(),
                    ),
                    span: field_name.span.clone(),
                });
            }

            let expected_field_type = expected_type
                .and_then(|et| et.kind.get_field(&field_name.name).map(|(_, ty)| ty));

            let val_id = self.build_expr(field_expr, expected_field_type);
            field_values.push((field_name, val_id));
        }

        let result = self.emit_struct_init(field_values);
        self.check_expected(result, span, expected_type)
    }
}
