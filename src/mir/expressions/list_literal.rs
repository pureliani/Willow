use crate::{
    ast::{expr::Expr, Span},
    compile::interner::GenericSubstitutions,
    globals::COMMON_IDENTIFIERS,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{SpannedType, StructKind, Type},
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_list_literal_expr(
        &mut self,
        expr_span: Span,
        items: Vec<Expr>,
        expected_type: Option<&SpannedType>,
        substitutions: &GenericSubstitutions,
    ) -> ValueId {
        let mut item_values = Vec::with_capacity(items.len());
        let mut element_types = Vec::with_capacity(items.len());

        let expected_element_type = if let Some(et) = expected_type {
            let mut ty_id = et.id;

            if self.types.is_pointer(ty_id) {
                ty_id = self.types.unwrap_ptr(ty_id);
            }

            if let Type::Struct(StructKind::ListHeader(elem_ty)) =
                self.types.resolve(ty_id)
            {
                Some(SpannedType {
                    id: elem_ty,
                    span: et.span.clone(),
                })
            } else {
                None
            }
        } else {
            None
        };

        for item in items {
            let val_id =
                self.build_expr(item, expected_element_type.as_ref(), substitutions);
            let ty = self.get_value_type(val_id);

            item_values.push(val_id);
            element_types.push(ty);
        }

        let element_type_id = if let Some(et) = &expected_element_type {
            et.id
        } else if element_types.is_empty() {
            self.types.unknown()
        } else {
            self.types.make_union(element_types)
        };

        let mut adjusted_items = Vec::with_capacity(item_values.len());
        for val_id in item_values {
            let val_ty = self.get_value_type(val_id);
            if val_ty == element_type_id {
                adjusted_items.push(val_id);
            } else {
                match self.compute_type_adjustment(val_ty, element_type_id, false) {
                    Ok(adj) => {
                        let adjusted = self.apply_adjustment(
                            val_id,
                            adj,
                            element_type_id,
                            expr_span.clone(),
                        );
                        adjusted_items.push(adjusted);
                    }
                    Err(_) => {
                        panic!(
                            "INTERNAL COMPILER ERROR: List item not assignable to \
                             element type"
                        );
                    }
                }
            }
        }

        let len_val = self.emit_number(NumberKind::USize(adjusted_items.len()));
        let buffer_ptr = self.emit_heap_alloc(element_type_id, len_val);

        for (i, item_val) in adjusted_items.into_iter().enumerate() {
            let idx_val = self.emit_number(NumberKind::USize(i));
            let item_ptr = self.ptr_offset(buffer_ptr, idx_val);
            self.emit_store(item_ptr, item_val);
        }

        let header_struct_kind = StructKind::ListHeader(element_type_id);
        let header_ty_id = self.types.intern(&Type::Struct(header_struct_kind));

        let count_val = self.emit_number(NumberKind::USize(1));
        let header_ptr = self.emit_heap_alloc(header_ty_id, count_val);

        let len_ptr = self.get_field_ptr(header_ptr, COMMON_IDENTIFIERS.len);
        self.emit_store(len_ptr, len_val);

        let cap_ptr = self.get_field_ptr(header_ptr, COMMON_IDENTIFIERS.cap);
        self.emit_store(cap_ptr, len_val);

        let buf_ptr_ptr = self.get_field_ptr(header_ptr, COMMON_IDENTIFIERS.ptr);
        self.emit_store(buf_ptr_ptr, buffer_ptr);

        self.check_expected(header_ptr, expr_span, expected_type)
    }
}
