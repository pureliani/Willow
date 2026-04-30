use std::collections::BTreeMap;

use crate::{
    ast::decl::{Declaration, FnDecl},
    globals::STRING_INTERNER,
    hir::{
        builders::{Builder, InBlock, InModule},
        errors::{SemanticError, SemanticErrorKind},
        instructions::{
            BasicBlockId, FunctionCFG, InstrId, InstructionKind, MakeLiteralKind,
            MemoryId,
        },
        utils::scope::ScopeKind,
    },
};

impl<'a> Builder<'a, InModule> {
    pub fn build_fn_body(&mut self, fn_decl: FnDecl) -> Result<(), SemanticError> {
        let is_valid_scope = match self.current_scope.kind() {
            ScopeKind::File => true,
            ScopeKind::GenericParams => self
                .current_scope
                .parent()
                .is_some_and(|p| p.is_file_scope()),
            _ => false,
        };

        if !is_valid_scope {
            return Err(SemanticError {
                kind: SemanticErrorKind::ClosuresNotSupportedYet,
                span: fn_decl.identifier.span.clone(),
            });
        }

        let decl_id = fn_decl.id;
        let raw_name = STRING_INTERNER.resolve(fn_decl.identifier.name);

        if raw_name == "main" {
            if let Some(entry_path) = &self.program.entry_path {
                if self.context.path != *entry_path {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::MainFunctionMustBeInEntryFile,
                        span: fn_decl.identifier.span.clone(),
                    });
                }
            }
            if !fn_decl.params.is_empty() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MainFunctionCannotHaveParameters,
                    span: fn_decl.identifier.span.clone(),
                });
            }
        }

        let cfg = FunctionCFG::new();
        self.program.cfgs.insert(decl_id, cfg);

        let mut current_def = BTreeMap::new();
        let mut incomplete_phis = BTreeMap::new();
        let mut current_memory_def = BTreeMap::new();
        let mut incomplete_memory_phis = BTreeMap::new();

        let entry_block_id = BasicBlockId(0);
        let mut fn_builder = Builder {
            context: InBlock {
                path: self.context.path.clone(),
                func_id: decl_id,
                block_id: entry_block_id,
            },
            program: self.program,
            errors: self.errors,
            current_scope: self
                .current_scope
                .enter(ScopeKind::FunctionBody, fn_decl.body.span.start),

            current_def: &mut current_def,
            incomplete_phis: &mut incomplete_phis,
            current_memory_def: &mut current_memory_def,
            incomplete_memory_phis: &mut incomplete_memory_phis,
        };

        fn_builder.write_memory(entry_block_id, MemoryId(0));

        for (i, param) in fn_decl.params.iter().enumerate() {
            let instr_id = fn_builder.push_instruction(
                InstructionKind::Param(i),
                param.identifier.span.clone(),
            );

            fn_builder.write_variable(entry_block_id, param.id, instr_id);

            fn_builder
                .program
                .declarations
                .insert(param.id, Declaration::Param(param.clone()));
            fn_builder
                .current_scope
                .map_name_to_symbol(param.identifier.name, param.id);
        }

        let (final_value, _) = fn_builder.build_codeblock_expr(fn_decl.body);

        if fn_builder.bb().terminator.is_none() {
            fn_builder.emit_return(final_value);
        }

        fn_builder.seal_block(entry_block_id);

        Ok(())
    }
}

impl<'a> Builder<'a, InBlock> {
    pub fn build_fn_expr(&mut self, fn_decl: FnDecl) -> InstrId {
        let id = fn_decl.id;
        let span = fn_decl.identifier.span.clone();

        if !fn_decl.generic_params.is_empty() {
            self.errors.push(SemanticError {
                span: span.clone(),
                kind: SemanticErrorKind::GenericClosuresNotSupported,
            });
            return self.push_instruction(
                InstructionKind::MakeLiteral(MakeLiteralKind::Unknown),
                span,
            );
        }

        if let Err(e) = self.as_module().build_fn_body(fn_decl) {
            self.errors.push(e);
        }

        self.push_instruction(InstructionKind::MakeLiteral(MakeLiteralKind::Fn(id)), span)
    }
}
