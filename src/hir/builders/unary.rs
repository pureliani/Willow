use crate::hir::{
    builders::{Builder, ConditionFact, InBlock, ValueId},
    instructions::{Instruction, UnaryInstr},
};

impl<'a> Builder<'a, InBlock> {
    fn emit_ineg(&mut self, src: ValueId) -> ValueId {
        let ty = self.get_value_type(src);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Unary(UnaryInstr::INeg { dest, src }));
        dest
    }

    fn emit_fneg(&mut self, src: ValueId) -> ValueId {
        let ty = self.get_value_type(src);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Unary(UnaryInstr::FNeg { dest, src }));
        dest
    }
}

impl<'a> Builder<'a, InBlock> {
    pub fn neg(&mut self, value: ValueId) -> ValueId {
        let value_type = self.get_value_type(value);
        let actual_value_type = self.types.unwrap_generic_bound(value_type);

        if actual_value_type == self.types.unknown() {
            return self.new_value_id(self.types.unknown());
        }

        if self.types.is_float(actual_value_type) {
            self.emit_fneg(value)
        } else if self.types.is_signed(actual_value_type) {
            self.emit_ineg(value)
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot apply unary negation `-` operator to \
                 this type"
            )
        }
    }

    pub fn not(&mut self, src: ValueId) -> ValueId {
        let value_type = self.get_value_type(src);
        let actual_value_type = self.types.unwrap_generic_bound(value_type);

        if actual_value_type == self.types.unknown() {
            return self.new_value_id(self.types.unknown());
        }

        if !self.types.is_bool(actual_value_type) {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot apply unary not `!` operator to this \
                 type"
            )
        }

        let bool_type = self.types.bool(None);
        let dest = self.new_value_id(bool_type);

        if let Some(facts) = self.condition_facts.get(&src).cloned() {
            let flipped: Vec<ConditionFact> = facts
                .into_iter()
                .map(|fact| ConditionFact {
                    place: fact.place,
                    on_true: fact.on_false,
                    on_false: fact.on_true,
                })
                .collect();

            self.condition_facts.insert(dest, flipped);
        }

        self.push_instruction(Instruction::Unary(UnaryInstr::BNot { dest, src }));

        dest
    }
}
