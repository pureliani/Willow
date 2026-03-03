use crate::{
    ast::Span,
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{Instruction, ListInstr},
        types::checked_type::Type,
        utils::{numeric::is_integer, points_to::PathSegment},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_list_init(&mut self, element_type: Type, items: Vec<ValueId>) -> ValueId {
        let dest = self.new_value_id(Type::List(Box::new(element_type.clone())));

        let alloc_id = self.ptg.new_alloc();
        self.ptg.bind_value_to_alloc(dest, alloc_id);

        for item_val in &items {
            if let Some(val_allocs) = self.ptg.value_locations.get(item_val).cloned() {
                for v_alloc in val_allocs {
                    self.ptg
                        .add_heap_edge(alloc_id, PathSegment::Index, v_alloc);
                }
            }
        }

        self.push_instruction(Instruction::List(ListInstr::Init {
            dest,
            element_type,
            items,
        }));

        dest
    }

    pub fn emit_list_get(
        &mut self,
        list: ValueId,
        index: ValueId,
        span: Span,
    ) -> ValueId {
        let list_type = self.get_value_type(list).clone();
        let index_type = self.get_value_type(index).clone();

        if !is_integer(&index_type) && !matches!(index_type, Type::Unknown) {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::ExpectedANumericOperand,
                span: span.clone(),
            });
        }

        match list_type {
            Type::List(inner) => {
                let result_type = Type::make_union([*inner, Type::Null]);

                let dest = self.new_value_id(result_type);
                self.push_instruction(Instruction::List(ListInstr::Get {
                    dest,
                    list,
                    index,
                }));

                self.ptg.read_path(dest, list, PathSegment::Index);

                dest
            }
            Type::Unknown => self.new_value_id(Type::Unknown),
            _ => self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::CannotIndex(list_type),
                span,
            }),
        }
    }

    pub fn emit_list_set(
        &mut self,
        list: ValueId,
        index: ValueId,
        value: ValueId,
        span: Span,
    ) -> ValueId {
        let list_type = self.get_value_type(list).clone();
        let index_type = self.get_value_type(index).clone();
        let value_type = self.get_value_type(value).clone();

        if !is_integer(&index_type) && !matches!(index_type, Type::Unknown) {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::ExpectedANumericOperand,
                span: span.clone(),
            });
        }

        match list_type {
            Type::List(inner) => {
                if value_type != *inner {
                    return self.report_error_and_get_poison(SemanticError {
                        kind: SemanticErrorKind::TypeMismatch {
                            expected: *inner,
                            received: value_type,
                        },
                        span,
                    });
                }

                let dest = self.new_value_id(Type::List(inner));
                self.push_instruction(Instruction::List(ListInstr::Set {
                    dest,
                    list,
                    index,
                    value,
                }));

                if let Some(allocs) = self.ptg.value_locations.get(&list).cloned() {
                    self.ptg.value_locations.insert(dest, allocs);
                }
                self.ptg.update_path(list, PathSegment::Index, value);

                dest
            }
            Type::Unknown => self.new_value_id(Type::Unknown),
            _ => self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::CannotIndex(list_type),
                span,
            }),
        }
    }

    pub fn emit_list_len(&mut self, list: ValueId, span: Span) -> ValueId {
        let list_type = self.get_value_type(list).clone();

        match list_type {
            Type::List(_) => {
                let dest = self.new_value_id(Type::USize);
                self.push_instruction(Instruction::List(ListInstr::Len { dest, list }));
                dest
            }
            Type::Unknown => self.new_value_id(Type::Unknown),
            _ => self.report_error_and_get_poison(SemanticError {
                kind: SemanticErrorKind::CannotGetLen(list_type),
                span,
            }),
        }
    }
}
