use std::collections::HashSet;

use crate::{
    ast::{expr::Expr, IdentifierNode, Span},
    compile::interner::StringId,
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        instructions::InstrId,
        types::{
            checked_declaration::CheckedParam,
            checked_type::{SpannedType, StructKind, Type},
        },
        utils::layout::{get_layout_of, pack_struct},
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
        let mut anonymous_params = Vec::new();

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

            evaluated_fields.push((field_name.clone(), val_id));

            anonymous_params.push(CheckedParam {
                identifier: field_name,
                ty: SpannedType {
                    id: val_ty,
                    span: Span::default(),
                },
            });
        }

        let struct_kind = pack_struct(
            StructKind::UserDefined(anonymous_params),
            self.types,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        let struct_ty_id = self.types.intern(&Type::Struct(struct_kind.clone()));

        let base_ptr = if !by_value {
            let layout = get_layout_of(
                &Type::Struct(struct_kind.clone()),
                self.types,
                self.program.target_ptr_size,
                self.program.target_ptr_align,
            );
            let count: usize = if layout.size == 0 { 0 } else { 1 };
            let count_val = self.emit_materialize(LiteralType::USize(count));
            self.emit_heap_alloc(struct_ty_id, count_val)
        } else {
            self.emit_stack_alloc(struct_ty_id, 1)
        };

        for (field_name, val_id) in evaluated_fields {
            let dest = self.get_field_ptr(base_ptr, field_name.name);
            self.emit_store(dest, val_id);
        }

        let result = if !by_value {
            base_ptr
        } else {
            self.emit_load(base_ptr)
        };

        result
    }
}
