use crate::{
    ast::{decl::VarDecl, DeclarationId, IdentifierNode, Span, SymbolId},
    compile::interner::{GenericSubstitutions, TypeId},
    hir::{
        builders::{Builder, InBlock},
        errors::{SemanticError, SemanticErrorKind},
        instructions::InstrId,
        types::{
            checked_declaration::{CheckedDeclaration, CheckedVarDecl},
            checked_type::Type,
        },
    },
    parse::DocAnnotation,
};

impl<'a> Builder<'a, InBlock> {
    pub fn declare_variable(
        &mut self,
        decl_id: DeclarationId,
        identifier: IdentifierNode,
        constraint: TypeId,
        initial_value: InstrId,
        constraint_span: Span,
        documentation: Option<DocAnnotation>,
    ) -> InstrId {
        let stack_ptr = self.emit_stack_alloc(constraint, 1);
        self.emit_store(stack_ptr, initial_value);

        let checked_var_decl = CheckedVarDecl {
            id: decl_id,
            identifier: identifier.clone(),
            documentation,
            constraint,
        };

        self.program
            .declarations
            .insert(decl_id, CheckedDeclaration::Var(checked_var_decl));

        self.own_declarations.insert(decl_id);

        self.current_scope
            .map_name_to_symbol(identifier.name, SymbolId::Concrete(decl_id));

        stack_ptr
    }

    pub fn build_var_decl(
        &mut self,
        var_decl: VarDecl,
        substitutions: &GenericSubstitutions,
    ) {
        if self.current_scope.is_file_scope() {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::CannotDeclareGlobalVariable,
                span: var_decl.identifier.span.clone(),
            });
            return;
        }

        let value_span = var_decl.value.span.clone();

        let constraint = var_decl
            .constraint
            .as_ref()
            .map(|c| self.check_type_annotation(c, substitutions));

        let mut value = self.build_expr(var_decl.value);

        let constraint_span = constraint
            .as_ref()
            .map(|c| c.span.clone())
            .unwrap_or(value_span);

        self.declare_variable(
            var_decl.id,
            var_decl.identifier,
            var_ty,
            value,
            constraint_span,
            var_decl.documentation,
        );
    }
}
