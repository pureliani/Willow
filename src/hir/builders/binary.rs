use crate::hir::{
    builders::{Builder, InBlock, ValueId},
    instructions::{BinaryInstr, Instruction},
};

impl<'a> Builder<'a, InBlock> {
    fn emit_add(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::IAdd { dest, lhs, rhs }));
        dest
    }

    fn emit_isub(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::ISub { dest, lhs, rhs }));
        dest
    }

    fn emit_imul(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::IMul { dest, lhs, rhs }));
        dest
    }

    fn emit_sdiv(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::SDiv { dest, lhs, rhs }));
        dest
    }

    fn emit_udiv(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::UDiv { dest, lhs, rhs }));
        dest
    }

    fn emit_srem(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::SRem { dest, lhs, rhs }));
        dest
    }

    fn emit_urem(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::URem { dest, lhs, rhs }));
        dest
    }

    fn emit_frem(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::FRem { dest, lhs, rhs }));
        dest
    }

    fn emit_fadd(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::FAdd { dest, lhs, rhs }));
        dest
    }

    fn emit_fsub(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::FSub { dest, lhs, rhs }));
        dest
    }

    fn emit_fmul(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::FMul { dest, lhs, rhs }));
        dest
    }

    fn emit_fdiv(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let ty = self.get_value_type(lhs);
        let dest = self.new_value_id(ty);
        self.push_instruction(Instruction::Binary(BinaryInstr::FDiv { dest, lhs, rhs }));
        dest
    }
}

impl<'a> Builder<'a, InBlock> {
    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn add(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        let effective_ty = self.types.unwrap_generic_bound(lhs_ty);

        if effective_ty == self.types.unknown() {
            return self.new_value_id(self.types.unknown());
        }

        if self.types.is_float(effective_ty) {
            self.emit_fadd(lhs, rhs)
        } else if self.types.is_integer(effective_ty) {
            self.emit_iadd(lhs, rhs)
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot use addition `+` operator on this type"
            )
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn sub(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        let effective_ty = self.types.unwrap_generic_bound(lhs_ty);

        if effective_ty == self.types.unknown() {
            return self.new_value_id(self.types.unknown());
        }

        if self.types.is_float(effective_ty) {
            self.emit_fsub(lhs, rhs)
        } else if self.types.is_integer(effective_ty) {
            self.emit_isub(lhs, rhs)
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot use subtraction `-` operator on this \
                 type"
            )
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn mul(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        let effective_ty = self.types.unwrap_generic_bound(lhs_ty);

        if effective_ty == self.types.unknown() {
            return self.new_value_id(self.types.unknown());
        }

        if self.types.is_float(effective_ty) {
            self.emit_fmul(lhs, rhs)
        } else if self.types.is_integer(effective_ty) {
            self.emit_imul(lhs, rhs)
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot use multiplication `*` operator on \
                 this type"
            )
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn div(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        let effective_ty = self.types.unwrap_generic_bound(lhs_ty);

        if effective_ty == self.types.unknown() {
            return self.new_value_id(self.types.unknown());
        }

        if self.types.is_float(effective_ty) {
            self.emit_fdiv(lhs, rhs)
        } else if self.types.is_signed(effective_ty) {
            self.emit_sdiv(lhs, rhs)
        } else if !self.types.is_signed(effective_ty) {
            self.emit_udiv(lhs, rhs)
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot use division `/` operator on this type"
            )
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn rem(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        let effective_ty = self.types.unwrap_generic_bound(lhs_ty);

        if effective_ty == self.types.unknown() {
            return self.new_value_id(self.types.unknown());
        }

        if self.types.is_float(effective_ty) {
            self.emit_frem(lhs, rhs)
        } else if self.types.is_signed(effective_ty) {
            self.emit_srem(lhs, rhs)
        } else if !self.types.is_signed(effective_ty) {
            self.emit_urem(lhs, rhs)
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: Cannot use remainder `%` operator on this type"
            )
        }
    }
}
