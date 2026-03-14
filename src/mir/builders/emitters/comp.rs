use crate::mir::{
    builders::{Builder, InBlock, ValueId},
    instructions::{CompInstr, Instruction, SelectInstr},
    types::checked_type::Type,
    utils::numeric::{is_float, is_integer, is_signed},
};

impl<'a> Builder<'a, InBlock> {
    fn emit_ieq(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::IEq { dest, lhs, rhs }));
        dest
    }

    fn emit_ineq(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::INeq { dest, lhs, rhs }));
        dest
    }

    fn emit_slt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::SLt { dest, lhs, rhs }));
        dest
    }

    fn emit_slte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::SLte { dest, lhs, rhs }));
        dest
    }

    fn emit_sgt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::SGt { dest, lhs, rhs }));
        dest
    }

    fn emit_sgte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::SGte { dest, lhs, rhs }));
        dest
    }

    fn emit_ult(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::ULt { dest, lhs, rhs }));
        dest
    }

    fn emit_ulte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::ULte { dest, lhs, rhs }));
        dest
    }

    fn emit_ugt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::UGt { dest, lhs, rhs }));
        dest
    }

    fn emit_ugte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::UGte { dest, lhs, rhs }));
        dest
    }

    fn emit_feq(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::FEq { dest, lhs, rhs }));
        dest
    }

    fn emit_fneq(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::FNeq { dest, lhs, rhs }));
        dest
    }

    fn emit_flt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::FLt { dest, lhs, rhs }));
        dest
    }

    fn emit_flte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::FLte { dest, lhs, rhs }));
        dest
    }

    fn emit_fgt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::FGt { dest, lhs, rhs }));
        dest
    }

    fn emit_fgte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dest = self.new_value_id(Type::Bool(None));
        self.push_instruction(Instruction::Comp(CompInstr::FGte { dest, lhs, rhs }));
        dest
    }
}

impl<'a> Builder<'a, InBlock> {
    /// **Assumption:** condition is type-adjusted to be a boolean before calling this method
    pub fn emit_select(
        &mut self,
        condition: ValueId,
        true_value: ValueId,
        false_value: ValueId,
    ) -> ValueId {
        let condition_type = self.get_value_type(condition);

        if !matches!(condition_type, &Type::Bool(_)) {
            panic!(
                "INTERNAL COMPILER ERROR: Select instruction expected the condition to \
                 be a boolean value"
            );
        }

        let true_value_type = self.get_value_type(true_value);
        let false_value_type = self.get_value_type(false_value);

        if true_value_type != false_value_type {
            panic!(
                "INTERNAL COMPILER ERROR: Select instruction expected both operands to \
                 have the same type"
            );
        }

        let dest = self.new_value_id(true_value_type.clone());
        self.push_instruction(Instruction::Select(SelectInstr {
            dest,
            cond: condition,
            true_val: true_value,
            false_val: false_value,
        }));

        dest
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn eq(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        if is_float(lhs_ty) {
            self.emit_feq(lhs, rhs)
        } else if is_integer(lhs_ty) {
            self.emit_ieq(lhs, rhs)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot use equality `=` comparison operator on this type");
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn neq(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        if is_float(lhs_ty) {
            self.emit_fneq(lhs, rhs)
        } else if is_integer(lhs_ty) {
            self.emit_ineq(lhs, rhs)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot use inequality `!=` comparison operator on this type");
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn lt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        if is_float(&lhs_ty) {
            self.emit_flt(lhs, rhs)
        } else if is_signed(&lhs_ty) {
            self.emit_slt(lhs, rhs)
        } else if !is_signed(&lhs_ty) {
            self.emit_ult(lhs, rhs)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot use less-than `<` comparison operator on this type")
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn lte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        if is_float(&lhs_ty) {
            self.emit_flte(lhs, rhs)
        } else if is_signed(&lhs_ty) {
            self.emit_slte(lhs, rhs)
        } else if !is_signed(&lhs_ty) {
            self.emit_ulte(lhs, rhs)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot use less-than-or-equal `<=` comparison operator on this type")
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn gt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        if is_float(&lhs_ty) {
            self.emit_fgt(lhs, rhs)
        } else if is_signed(&lhs_ty) {
            self.emit_sgt(lhs, rhs)
        } else if !is_signed(&lhs_ty) {
            self.emit_ugt(lhs, rhs)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot use greater-than `>` comparison operator on this type")
        }
    }

    /// **Assumption:** lhs and rhs types are type-adjusted before calling this method
    pub fn gte(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
        let lhs_ty = self.get_value_type(lhs);
        let rhs_ty = self.get_value_type(rhs);

        pretty_assertions::assert_eq!(
            lhs_ty,
            rhs_ty,
            "INTERNAL COMPILER ERROR: Expected lhs and rhs types to match"
        );

        if is_float(&lhs_ty) {
            self.emit_fgte(lhs, rhs)
        } else if is_signed(&lhs_ty) {
            self.emit_sgte(lhs, rhs)
        } else if !is_signed(&lhs_ty) {
            self.emit_ugte(lhs, rhs)
        } else {
            panic!("INTERNAL COMPILER ERROR: Cannot use greater-than-or-equal `>=` comparison operator on this type")
        }
    }
}
