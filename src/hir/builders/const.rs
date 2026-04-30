use crate::{
    ast::{DeclarationId, Span},
    compile::interner::StringId,
    hir::{
        builders::{Builder, InBlock},
        instructions::{InstrId, InstructionKind, MakeLiteralKind},
        types::checked_declaration::CheckedDeclaration,
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_number(&mut self, val: NumberKind, span: Span) -> InstrId {
        self.push_instruction(
            InstructionKind::MakeLiteral(MakeLiteralKind::Number(val)),
            span,
        )
    }

    pub fn emit_bool(&mut self, val: bool, span: Span) -> InstrId {
        self.push_instruction(
            InstructionKind::MakeLiteral(MakeLiteralKind::Bool(val)),
            span,
        )
    }

    pub fn emit_string(&mut self, val: StringId, span: Span) -> InstrId {
        self.push_instruction(
            InstructionKind::MakeLiteral(MakeLiteralKind::String(val)),
            span,
        )
    }

    pub fn emit_void(&mut self, span: Span) -> InstrId {
        self.push_instruction(InstructionKind::MakeLiteral(MakeLiteralKind::Void), span)
    }

    pub fn emit_null(&mut self, span: Span) -> InstrId {
        self.push_instruction(InstructionKind::MakeLiteral(MakeLiteralKind::Null), span)
    }

    pub fn emit_const_fn(&mut self, decl_id: DeclarationId, span: Span) -> InstrId {
        let decl = self
            .program
            .declarations
            .get(&decl_id)
            .expect("INTERNAL COMPILER ERROR: Function declaration not found");

        if !matches!(decl, CheckedDeclaration::Function(_)) {
            panic!("INTERNAL COMPILER ERROR: Declaration is not a function");
        }

        self.push_instruction(
            InstructionKind::MakeLiteral(MakeLiteralKind::Fn(decl_id)),
            span,
        )
    }
}
