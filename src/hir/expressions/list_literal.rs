use crate::{
    ast::{expr::Expr, Span},
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::checked_type::{SpannedType, Type},
        utils::adjustment::compute_type_adjustment,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_list_literal_expr(
        &mut self,
        expr_span: Span,
        items: Vec<Expr>,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let mut item_values = Vec::with_capacity(items.len());
        let mut element_types = Vec::with_capacity(items.len());

        let expected_element_type = if let Some(SpannedType {
            kind: Type::List(elem_type),
            span: _,
        }) = expected_type
        {
            Some(*elem_type.clone())
        } else {
            None
        };

        for item in items {
            let val_id = self.build_expr(item, expected_element_type.as_ref());
            let ty = self.get_value_type(val_id).clone();

            item_values.push(val_id);
            element_types.push(ty);
        }

        let element_type = SpannedType {
            kind: Type::make_union(element_types),
            span: expr_span.clone(),
        };

        let mut adjusted_items = Vec::with_capacity(item_values.len());
        for val_id in item_values {
            let val_ty = self.get_value_type(val_id).clone();
            if val_ty == element_type.kind {
                adjusted_items.push(val_id);
            } else {
                match compute_type_adjustment(&val_ty, &element_type.kind, false) {
                    Ok(adj) => {
                        let adjusted =
                            self.apply_adjustment(val_id, adj, element_type.kind.clone());
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

        let result = self.emit_list_init(element_type, adjusted_items);

        self.check_expected(result, expr_span, expected_type)
    }
}
