use std::collections::HashSet;

use crate::{
    ast::{expr::Expr, IdentifierNode, Span},
    compile::interner::StringId,
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{InstrId, InstructionKind, StructInitInstr},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_struct_init_expr(
        &mut self,
        span: Span,
        fields: Vec<(IdentifierNode, Expr)>,
        by_value: bool,
    ) -> InstrId {
        let mut seen_names: HashSet<StringId> = HashSet::new();
        let mut evaluated_fields = Vec::with_capacity(fields.len());

        for (field_name, field_expr) in fields {
            if !seen_names.insert(field_name.name) {
                self.errors.push(SemanticError {
                    kind: SemanticErrorKind::DuplicateStructFieldInitializer(
                        field_name.clone(),
                    ),
                    span: field_name.span.clone(),
                });
            }

            let val_id = self.build_expr(field_expr);
            evaluated_fields.push((field_name, val_id));
        }

        self.push_instruction(
            InstructionKind::StructInit(StructInitInstr {
                fields: evaluated_fields,
                by_value,
            }),
            span,
        )
    }
}
