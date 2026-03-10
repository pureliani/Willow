use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        types::{
            checked_type::{LiteralType, SpannedType, Type},
            ordered_number_kind::OrderedNumberKind,
        },
    },
    tokenize::NumberKind,
};

fn safely_widen(value: &NumberKind, target: &Type) -> Option<NumberKind> {
    match (value, target) {
        (NumberKind::I8(v), Type::I16) => Some(NumberKind::I16(*v as i16)),
        (NumberKind::I8(v), Type::I32) => Some(NumberKind::I32(*v as i32)),
        (NumberKind::I8(v), Type::I64) => Some(NumberKind::I64(*v as i64)),
        (NumberKind::I8(v), Type::ISize) => Some(NumberKind::ISize(*v as isize)),

        (NumberKind::I16(v), Type::I32) => Some(NumberKind::I32(*v as i32)),
        (NumberKind::I16(v), Type::I64) => Some(NumberKind::I64(*v as i64)),
        (NumberKind::I16(v), Type::ISize) => Some(NumberKind::ISize(*v as isize)),

        (NumberKind::I32(v), Type::I64) => Some(NumberKind::I64(*v as i64)),
        (NumberKind::I32(v), Type::ISize) => Some(NumberKind::ISize(*v as isize)),

        (NumberKind::U8(v), Type::U16) => Some(NumberKind::U16(*v as u16)),
        (NumberKind::U8(v), Type::U32) => Some(NumberKind::U32(*v as u32)),
        (NumberKind::U8(v), Type::U64) => Some(NumberKind::U64(*v as u64)),
        (NumberKind::U8(v), Type::USize) => Some(NumberKind::USize(*v as usize)),

        (NumberKind::U16(v), Type::U32) => Some(NumberKind::U32(*v as u32)),
        (NumberKind::U16(v), Type::U64) => Some(NumberKind::U64(*v as u64)),
        (NumberKind::U16(v), Type::USize) => Some(NumberKind::USize(*v as usize)),

        (NumberKind::U32(v), Type::U64) => Some(NumberKind::U64(*v as u64)),
        (NumberKind::U32(v), Type::USize) => Some(NumberKind::USize(*v as usize)),

        (NumberKind::F32(v), Type::F64) => Some(NumberKind::F64(*v as f64)),

        _ => None,
    }
}

impl<'a> Builder<'a, InBlock> {
    pub fn build_number_expr(
        &mut self,
        span: Span,
        value: NumberKind,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let widened = Type::from_number_kind(&value);
        let literal = Type::Literal(LiteralType::Number(OrderedNumberKind(value)));

        if let Some(et) = expected_type {
            if et.kind == literal {
                return self.emit_number_literal(value);
            }

            if et.kind == widened {
                return self.emit_number(value);
            }

            if let Some(widened_value) = safely_widen(&value, &et.kind) {
                return self.emit_number(widened_value);
            }

            if let Some(variants) = et.kind.get_narrowed_variants() {
                if variants.contains(&literal) {
                    let val = self.emit_number_literal(value);
                    return self.emit_wrap_in_union(val, &et.kind);
                }

                if variants.contains(&widened) {
                    let val = self.emit_number(value);
                    return self.emit_wrap_in_union(val, &et.kind);
                }

                let widenable: Vec<&Type> = variants
                    .iter()
                    .filter(|v| safely_widen(&value, v).is_some())
                    .collect();

                if widenable.len() == 1 {
                    let widened_value = safely_widen(&value, widenable[0]).unwrap();
                    let val = self.emit_number(widened_value);
                    return self.emit_wrap_in_union(val, &et.kind);
                }
            }
        }

        self.emit_number_literal(value)
    }
}
