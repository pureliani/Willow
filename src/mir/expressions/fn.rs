use std::collections::{HashMap, HashSet};

use crate::{
    ast::{
        decl::{FnDecl, FnDeclBody},
        Span,
    },
    globals::{next_block_id, next_declaration_id, STRING_INTERNER},
    mir::{
        builders::{
            BasicBlock, Builder, ExpectBody, FunctionBodyKind, FunctionCFG, InBlock,
            InModule, ValueId,
        },
        errors::{SemanticError, SemanticErrorKind},
        instructions::Terminator,
        types::{
            checked_declaration::{
                CheckedDeclaration, CheckedVarDecl, FunctionEffects, ParamMutation,
            },
            checked_type::{SpannedType, Type},
        },
        utils::{points_to::PointsToGraph, scope::ScopeKind},
    },
};

impl<'a> Builder<'a, InModule> {
    pub fn build_fn_body(&mut self, fn_decl: FnDecl) -> Result<(), SemanticError> {
        if !self.current_scope.is_file_scope() {
            return Err(SemanticError {
                kind: SemanticErrorKind::ClosuresNotSupportedYet,
                span: fn_decl.identifier.span.clone(),
            });
        }

        let FnDecl {
            id: decl_id,
            identifier,
            body: body_variant,
            ..
        } = fn_decl;

        let body = match body_variant {
            FnDeclBody::External => return Ok(()),
            FnDeclBody::Internal(block) => block,
        };

        let raw_name = STRING_INTERNER.resolve(identifier.name);
        if raw_name == "main" {
            if let Some(entry_path) = &self.program.entry_path {
                if self.context.path != *entry_path {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::MainFunctionMustBeInEntryFile,
                        span: identifier.span.clone(),
                    });
                }
            }

            let func = match self.program.declarations.get(&decl_id) {
                Some(CheckedDeclaration::Function(f)) => f,
                _ => panic!("INTERNAL COMPILER ERROR: Function declaration not found"),
            };

            if !func.params.is_empty() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MainFunctionCannotHaveParameters,
                    span: identifier.span.clone(),
                });
            }

            let ret = &func.return_type.kind;
            if !matches!(ret, Type::Void | Type::I32) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MainFunctionInvalidReturnType,
                    span: identifier.span.clone(),
                });
            }
        }

        let entry_block_id = next_block_id();

        if let Some(CheckedDeclaration::Function(func)) =
            self.program.declarations.get_mut(&decl_id)
        {
            let cfg = FunctionCFG {
                entry_block: entry_block_id,
                blocks: HashMap::new(),
                value_definitions: HashMap::new(),
                ptg: PointsToGraph::new(),
                effects: FunctionEffects::default(),
            };
            func.body = FunctionBodyKind::Internal(cfg);
        } else {
            panic!("INTERNAL COMPILER ERROR: Function declaration not found");
        }

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
                .enter(ScopeKind::FunctionBody, body.span.start),
            condition_facts: self.condition_facts,
            current_facts: self.current_facts,
            incomplete_fact_merges: self.incomplete_fact_merges,
            ptg: self.ptg,
            aliases: self.aliases,
            types: self.types,
        };

        let entry_bb = BasicBlock {
            id: entry_block_id,
            instructions: vec![],
            terminator: None,
            predecessors: HashSet::new(),
            phis: HashMap::new(),
            sealed: true,
        };
        fn_builder
            .get_fn()
            .expect_body()
            .blocks
            .insert(entry_block_id, entry_bb);

        let param_count = fn_builder.get_fn().params.len();

        for i in 0..param_count {
            let (param_ty, param_ident) = {
                let p = &fn_builder.get_fn().params[i];
                (p.ty.clone(), p.identifier.clone())
            };

            let val_id = fn_builder.new_value_id(param_ty.kind.clone());
            let var_decl_id = next_declaration_id();

            let decl = CheckedVarDecl {
                id: var_decl_id,
                identifier: param_ident.clone(),
                documentation: None,
                constraint: param_ty,
            };

            fn_builder.write_variable(var_decl_id, entry_block_id, val_id);

            fn_builder
                .program
                .declarations
                .insert(var_decl_id, CheckedDeclaration::Var(decl));

            fn_builder
                .current_scope
                .map_name_to_decl(param_ident.name, var_decl_id);

            let param = &mut fn_builder.get_fn().params[i];
            param.decl_id = Some(var_decl_id);
            param.value_id = Some(val_id);
        }

        let return_type = fn_builder.get_fn().return_type.clone();

        let (final_value, _) =
            fn_builder.build_codeblock_expr(body, Some(&return_type), false);
        if fn_builder.bb().terminator.is_none() {
            fn_builder.emit_return(final_value);
        }

        let effects = fn_builder.compute_effects(&identifier.span);
        fn_builder.get_fn().expect_body().effects = effects;

        Ok(())
    }
}

impl<'a> Builder<'a, InBlock> {
    pub fn build_fn_expr(
        &mut self,
        fn_decl: FnDecl,
        expected_type: Option<&SpannedType>,
    ) -> ValueId {
        let id = fn_decl.id;
        let span = fn_decl.identifier.span.clone();

        match self.as_module().build_fn_body(fn_decl) {
            Ok(_) => {}
            Err(e) => self.errors.push(e),
        };

        let result = self.emit_const_fn(id);
        self.check_expected(result, span, expected_type)
    }

    fn compute_effects(&mut self, fn_span: &Span) -> FunctionEffects {
        let func = self.get_fn();

        let return_block_ids: Vec<_> = func
            .expect_body()
            .blocks
            .iter()
            .filter_map(|(id, bb)| {
                if matches!(bb.terminator, Some(Terminator::Return { .. })) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();

        if return_block_ids.is_empty() {
            return FunctionEffects::default();
        }

        let params: Vec<_> = func.params.clone();
        let mut mutations = Vec::new();

        for (i, param) in params.iter().enumerate() {
            let declared_type = &param.ty;

            let param_decl_id = param.decl_id.expect(
                "INTERNAL COMPILER ERROR: Param decl_id not set during effect \
                 computation",
            );

            let mut exit_types: Vec<Type> = Vec::new();
            let mut any_changed = false;

            for &block_id in &return_block_ids {
                let val = self.read_variable(param_decl_id, block_id, fn_span.clone());
                let ty = self.get_value_type(val).clone();

                if ty != declared_type.kind {
                    any_changed = true;
                }

                exit_types.push(ty);
            }

            if any_changed {
                let exit_type = Type::make_union(exit_types);
                mutations.push(ParamMutation {
                    param_index: i,
                    exit_type,
                });
            }
        }

        FunctionEffects { mutations }
    }
}
