use crate::mir::{
    builders::{Builder, InBlock, TypePredicate, ValueId},
    instructions::{Instruction, UnaryInstr},
};

impl<'a> Builder<'a, InBlock> {
    fn emit_ineg(&mut self, src: ValueId) -> ValueId {
        let ty = self.get_value_type(src).clone();
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Unary(UnaryInstr::INeg { dest, src }));
        dest
    }

    fn emit_fneg(&mut self, src: ValueId) -> ValueId {
        let ty = self.get_value_type(src).clone();
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Unary(UnaryInstr::FNeg { dest, src }));
        dest
    }
}
impl<'a> Builder<'a, InBlock> {
    pub fn neg(&mut self, value: ValueId) -> ValueId {
        let value_type = self.get_value_type(value);

        if self.types.is_float(value_type) {
            self.emit_fneg(value)
        } else if self.types.is_signed(value_type) {
            self.emit_ineg(value)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot apply unary negation `-` operator to this type")
        }
    }

    pub fn not(&mut self, src: ValueId) -> ValueId {
        let value_type = self.get_value_type(src);

        if !self.types.is_bool(value_type) {
            panic!("INTERNAL COMPILER ERROR: Cannot apply unary not `!` operator to this type")
        }

        let bool_type = self.types.bool(None);
        let dest = self.new_value_id(bool_type);

        if let Some(preds) = self.condition_facts.get(&src).cloned() {
            let flipped: Vec<TypePredicate> = preds
                .into_iter()
                .map(|pred| TypePredicate {
                    decl_id: pred.decl_id,
                    on_true_type: pred.on_false_type,
                    on_false_type: pred.on_true_type,
                })
                .collect();

            self.type_predicates.insert(dest, flipped);
        }

        self.push_instruction(Instruction::Unary(UnaryInstr::BNot { dest, src }));

        dest
    }
}
