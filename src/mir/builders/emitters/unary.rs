use crate::mir::{
    builders::{Builder, InBlock, ValueId},
    errors::SemanticError,
    instructions::{Instruction, UnaryInstr},
    types::checked_type::Type,
    utils::numeric::{is_float, is_signed},
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

    fn emit_bnot(&mut self, src: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Unary(UnaryInstr::BNot { dest, src }));
        dest
    }
}
impl<'a> Builder<'a, InBlock> {
    pub fn neg(&mut self, src: ValueId) -> ValueId {
        let ty = self.get_value_type(src);

        if is_float(ty) {
            self.emit_fneg(src)
        } else if is_signed(ty) {
            self.emit_ineg(src)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot apply unary negation `-` operator to this type")
        }
    }

    pub fn not(&mut self, src: ValueId) -> Result<ValueId, SemanticError> {
        let ty = self.get_value_type(src);

        if !matches!(ty, &Type::Bool(_)) {
            panic!("INTERNAL COMPILER ERROR: Cannot apply unary not `-` operator to this type")
        }

        Ok(self.emit_bnot(src))
    }
}
