use crate::{
    ast::DeclarationId,
    compile::interner::StringId,
    mir::{
        builders::{Builder, InBlock, ValueId},
        types::{
            checked_declaration::{CheckedDeclaration, FnType},
            checked_type::{StructKind, Type},
        },
    },
    tokenize::NumberKind,
};

impl<'a> Builder<'a, InBlock> {
    pub fn emit_number(&mut self, val: NumberKind) -> ValueId {
        let ty = self.types.from_number_kind(&val);
        self.new_value_id(ty)
    }

    pub fn emit_bool(&mut self, val: bool) -> ValueId {
        let ty = self.types.bool(Some(val));
        self.new_value_id(ty)
    }

    pub fn emit_string(&mut self, val: StringId) -> ValueId {
        let ty = self
            .types
            .intern(&Type::Struct(StructKind::StringHeader(Some(val))));
        self.new_value_id(ty)
    }

    pub fn emit_void(&mut self) -> ValueId {
        let ty = self.types.void();
        self.new_value_id(ty)
    }

    pub fn emit_null(&mut self) -> ValueId {
        let ty = self.types.null();
        self.new_value_id(ty)
    }

    pub fn emit_const_fn(&mut self, decl_id: DeclarationId) -> ValueId {
        let decl = self
            .program
            .declarations
            .get(&decl_id)
            .expect("INTERNAL COMPILER ERROR: Function declaration not found");

        if !matches!(decl, CheckedDeclaration::Function(_)) {
            panic!("INTERNAL COMPILER ERROR: Declaration is not a function");
        }

        let ty = self.types.intern(&Type::Fn(FnType::Direct(decl_id)));

        self.new_value_id(ty)
    }
}
