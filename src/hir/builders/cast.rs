use crate::{
    compile::interner::TypeId,
    hir::{
        builders::{Builder, InBlock, ValueId},
        instructions::{CastInstr, Instruction, MaterializeInstr},
        types::checked_type::LiteralType,
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_fext(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::FExt { dest, src }));
        dest
    }

    pub fn emit_ftrunc(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::FTrunc { dest, src }));
        dest
    }

    pub fn emit_trunc(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::Trunc { dest, src }));
        dest
    }

    pub fn emit_sitof(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::SIToF { dest, src }));
        dest
    }

    pub fn emit_uitof(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::UIToF { dest, src }));
        dest
    }

    pub fn emit_ftosi(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::FToSI { dest, src }));
        dest
    }

    pub fn emit_ftoui(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::FToUI { dest, src }));
        dest
    }

    pub fn emit_sext(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::SExt { dest, src }));
        dest
    }

    pub fn emit_zext(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::ZExt { dest, src }));
        dest
    }

    pub fn emit_bitcast(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::BitCast { dest, src }));
        dest
    }

    pub fn emit_materialize(&mut self, literal_type: LiteralType) -> ValueId {
        match literal_type {
            LiteralType::Void
            | LiteralType::Never
            | LiteralType::Unknown
            | LiteralType::Null => {
                panic!(
                    "INTERNAL COMPILER ERROR: Cannot materialize literal type {:?}",
                    literal_type
                );
            }
            _ => {}
        }

        let widened_type = self.types.widen_literal(literal_type);

        let dest = self.new_value_id(widened_type);

        self.push_instruction(Instruction::Materialize(MaterializeInstr {
            dest,
            literal_type,
        }));

        dest
    }
}
