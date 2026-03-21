use crate::{
    compile::interner::TypeId,
    mir::{
        builders::{Builder, InBlock, ValueId},
        instructions::{CastInstr, Instruction},
        utils::type_to_string::type_to_string,
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
        let src_ty = self.get_value_type(src);
        if src_ty != target_ty {
            panic!(
                "INTERNAL COMPILER ERROR: Tried a bitcast to structurally incompatible \
                 type, from {} to {}",
                type_to_string(src_ty),
                type_to_string(target_ty)
            );
        }

        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::BitCast { dest, src }));
        dest
    }

    pub fn emit_bitcast_unsafe(&mut self, src: ValueId, target_ty: TypeId) -> ValueId {
        let src_ty = self.get_value_type(src);

        if !self.types.is_pointer(src_ty) || !self.types.is_pointer(target_ty) {
            panic!(
                "INTERNAL COMPILER ERROR: emit_bitcast_unsafe should only be used for \
                 pointer-to-pointer casts"
            );
        }

        let dest = self.new_value_id(target_ty);
        self.push_instruction(Instruction::Cast(CastInstr::BitCast { dest, src }));
        dest
    }
}
