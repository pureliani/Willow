use std::collections::BTreeSet;

use crate::{
    ast::{decl::VarDecl, DeclarationId, IdentifierNode, Span, SymbolId},
    compile::interner::{GenericSubstitutions, TypeId},
    hir::{
        builders::{Builder, InBlock, ValueId},
        errors::{SemanticError, SemanticErrorKind},
        types::{
            checked_declaration::{CheckedDeclaration, CheckedVarDecl},
            checked_type::Type,
        },
        utils::{
            facts::{narrowed_type::NarrowedTypeFact, FactSet},
            place::Place,
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
        initial_value: ValueId,
        constraint_span: Span,
        documentation: Option<DocAnnotation>,
    ) -> ValueId {
        let stack_ptr = self.emit_stack_alloc(constraint, 1);
        self.emit_store(stack_ptr, initial_value);

        let value_type = self.get_value_type(initial_value);
        let mut initial_facts = FactSet::new();

        if let Some(variants) = self.types.get_union_variants(value_type) {
            initial_facts.insert(NarrowedTypeFact { variants });
        } else {
            initial_facts.insert(NarrowedTypeFact {
                variants: BTreeSet::from([value_type]),
            });
        }

        self.write_fact(self.context.block_id, &Place::Var(decl_id), initial_facts);

        let checked_var_decl = CheckedVarDecl {
            id: decl_id,
            identifier: identifier.clone(),
            documentation,
            stack_ptr,
            constraint_span,
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

        let mut value =
            self.build_expr(var_decl.value, constraint.as_ref(), substitutions);
        let mut value_type = self.get_value_type(value);

        if constraint.is_none() {
            if let Type::Literal(lit) = self.types.resolve(value_type) {
                let widened_ty = self.types.widen_literal(lit);
                if widened_ty != value_type {
                    value = self.emit_materialize(lit);
                    value_type = widened_ty;
                }
            }
        }

        let var_ty = constraint.as_ref().map(|c| c.id).unwrap_or(value_type);
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
