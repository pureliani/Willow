use crate::{
    ast::decl::{Declaration, VarDecl},
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn build_var_decl(&mut self, var_decl: VarDecl) {
        if self.current_scope.is_file_scope() {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::CannotDeclareGlobalVariable,
                span: var_decl.identifier.span.clone(),
            });
            return;
        }

        let value_id = self.build_expr(var_decl.value.clone());
        self.write_variable(self.context.block_id, var_decl.id, value_id);

        let decl_id = var_decl.id;
        let name = var_decl.identifier.name;

        self.program
            .declarations
            .insert(decl_id, Declaration::Var(var_decl));

        self.current_scope.map_name_to_symbol(name, decl_id);
    }
}
