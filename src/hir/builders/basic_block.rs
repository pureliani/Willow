use std::collections::HashSet;

use crate::{
    ast::{DeclarationId, Span},
    globals::next_value_id,
    hir::{
        builders::{
            BasicBlock, BasicBlockId, Builder, Function, InBlock, InFunction, InGlobal,
            InModule, PhiSource, ValueId,
        },
        instructions::{CastInstr, Instruction, Terminator},
        types::{checked_declaration::CheckedDeclaration, checked_type::Type},
        utils::{check_assignable::Adjustment, type_to_string::type_to_string},
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn as_program(&mut self) -> Builder<'_, InGlobal> {
        Builder {
            context: InGlobal,
            program: self.program,
            errors: self.errors,
            current_scope: self.current_scope.clone(),
            current_defs: self.current_defs,
            incomplete_phis: self.incomplete_phis,
            type_predicates: self.type_predicates,
            ptg: self.ptg,
        }
    }

    pub fn as_module(&mut self) -> Builder<'_, InModule> {
        Builder {
            context: InModule {
                path: self.context.path.clone(),
            },
            program: self.program,
            errors: self.errors,
            current_scope: self.current_scope.clone(),
            current_defs: self.current_defs,
            incomplete_phis: self.incomplete_phis,
            type_predicates: self.type_predicates,
            ptg: self.ptg,
        }
    }

    pub fn as_fn(&mut self) -> Builder<'_, InFunction> {
        Builder {
            context: InFunction {
                path: self.context.path.clone(),
                func_id: self.context.func_id,
            },
            program: self.program,
            errors: self.errors,
            current_scope: self.current_scope.clone(),
            current_defs: self.current_defs,
            incomplete_phis: self.incomplete_phis,
            type_predicates: self.type_predicates,
            ptg: self.ptg,
        }
    }

    pub fn bb_mut(&mut self) -> &mut BasicBlock {
        self.get_bb_mut(self.context.block_id)
    }

    pub fn get_bb_mut(&mut self, block_id: BasicBlockId) -> &mut BasicBlock {
        let func_id = self.context.func_id;

        let decl = self
            .program
            .declarations
            .get_mut(&func_id)
            .expect("INTERNAL COMPILER ERROR: Function not found");

        match decl {
            CheckedDeclaration::Function(f) => f
                .blocks
                .get_mut(&block_id)
                .expect("INTERNAL COMPILER ERROR: Block not found"),
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        }
    }

    pub fn get_bb(&self, block_id: BasicBlockId) -> &BasicBlock {
        let func_id = self.context.func_id;

        let decl = self
            .program
            .declarations
            .get(&func_id)
            .expect("INTERNAL COMPILER ERROR: Function not found");

        match decl {
            CheckedDeclaration::Function(f) => f
                .blocks
                .get(&block_id)
                .expect("INTERNAL COMPILER ERROR: Block not found"),
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        }
    }

    pub fn bb(&self) -> &BasicBlock {
        self.get_bb(self.context.block_id)
    }

    pub fn get_fn(&mut self) -> &mut Function {
        let func_id = self.context.func_id;

        match self.program.declarations.get_mut(&func_id).unwrap() {
            CheckedDeclaration::Function(f) => f,
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        }
    }

    pub fn get_value_type(&self, id: ValueId) -> &Type {
        self.program.value_types.get(&id).unwrap_or_else(|| {
            panic!("INTERNAL COMPILER ERROR: ValueId({}) has no type", id.0)
        })
    }

    pub fn apply_adjustment(
        &mut self,
        src: ValueId,
        adjustment: Adjustment,
        target_type: Type,
    ) -> ValueId {
        match adjustment {
            Adjustment::Identity => src,
            _ => {
                let dest = self.new_value_id(target_type);
                self.push_instruction(Instruction::Cast(CastInstr {
                    src,
                    dest,
                    op: adjustment,
                }));
                dest
            }
        }
    }

    pub fn write_variable(
        &mut self,
        variable: DeclarationId,
        block: BasicBlockId,
        value: ValueId,
    ) {
        self.current_defs
            .entry(block)
            .or_default()
            .insert(variable, value);
    }

    pub fn read_variable(
        &mut self,
        variable: DeclarationId,
        block: BasicBlockId,
        span: Span,
    ) -> ValueId {
        if let Some(block_defs) = self.current_defs.get(&block) {
            if let Some(val) = block_defs.get(&variable) {
                return *val;
            }
        }
        self.read_variable_recursive(variable, block, span)
    }

    fn read_variable_recursive(
        &mut self,
        variable: DeclarationId,
        block: BasicBlockId,
        span: Span,
    ) -> ValueId {
        let val_id;
        let sealed = self.get_bb(block).sealed;
        let predecessors: Vec<BasicBlockId> =
            self.get_bb(block).predecessors.iter().cloned().collect();

        if !sealed {
            val_id = self.new_value_id(Type::Unknown);
            self.incomplete_phis.entry(block).or_default().push((
                val_id,
                variable,
                span.clone(),
            ));
        } else if predecessors.len() == 1 {
            val_id = self.read_variable(variable, predecessors[0], span.clone());
        } else if predecessors.is_empty() {
            panic!("INTERNAL COMPILER ERROR: Uninitialized local variable read");
        } else {
            val_id = self.new_value_id(Type::Unknown);
            self.write_variable(variable, block, val_id);
            self.resolve_phi(block, val_id, variable, span.clone());
        }

        self.write_variable(variable, block, val_id);
        val_id
    }

    pub fn insert_phi(
        &mut self,
        basic_block_id: BasicBlockId,
        phi_id: ValueId,
        sources: HashSet<PhiSource>,
    ) {
        assert!(
            !sources.is_empty(),
            "Phi node must have at least one source"
        );

        let first_source = sources.iter().next().unwrap();
        let expected_type = self.get_value_type(first_source.value);

        for source in &sources {
            let current_type = self.get_value_type(source.value);

            if expected_type != current_type {
                panic!(
                    "INTERNAL COMPILER ERROR: Phi node type mismatch.\nPhi ID: \
                     {:?}\nBlock ID: {:?}\nExpected Type: {}\nFound Type: {} (from \
                     block {:?})",
                    phi_id,
                    basic_block_id,
                    type_to_string(expected_type),
                    type_to_string(current_type),
                    source.from
                );
            }
        }

        self.get_bb_mut(basic_block_id).phis.insert(phi_id, sources);
    }

    pub fn resolve_phi(
        &mut self,
        block_id: BasicBlockId,
        phi_id: ValueId,
        variable_id: DeclarationId,
        span: Span,
    ) {
        let predecessors: Vec<BasicBlockId> =
            self.get_bb(block_id).predecessors.iter().cloned().collect();

        let mut incoming_values = Vec::new();
        let mut incoming_types = Vec::new();

        for pred in &predecessors {
            let val = self.read_variable(variable_id, *pred, span.clone());
            incoming_values.push((*pred, val));
            incoming_types.push(self.get_value_type(val).clone());
        }

        let unified_type = Type::make_union(incoming_types);

        let phi_ty = self.program.value_types.get_mut(&phi_id).expect(
            "INTERNAL COMPILER ERROR: Expected the type for phi_id to be defined",
        );
        *phi_ty = unified_type.clone();

        let mut final_sources = HashSet::new();

        for (pred_block, val) in incoming_values {
            let val_type = self.get_value_type(val);

            if val_type == &unified_type {
                final_sources.insert(PhiSource {
                    from: pred_block,
                    value: val,
                });
            } else {
                let insertion_block = self.split_critical_edge(pred_block, block_id);

                let casted_val =
                    self.emit_cast_internal(insertion_block, val, unified_type.clone());

                final_sources.insert(PhiSource {
                    from: insertion_block,
                    value: casted_val,
                });
            }
        }

        self.insert_phi(block_id, phi_id, final_sources.clone());

        let source_values: Vec<ValueId> =
            final_sources.into_iter().map(|s| s.value).collect();
        self.ptg.merge_values(phi_id, &source_values);
    }

    fn emit_cast_internal(
        &mut self,
        block_id: BasicBlockId,
        src: ValueId,
        target_type: Type,
    ) -> ValueId {
        let old_block = self.context.block_id;
        self.use_basic_block(block_id);

        let terminator = self.bb_mut().terminator.take();

        let dest = self.new_value_id(target_type);
        self.push_instruction(Instruction::Cast(CastInstr { src, dest }));

        self.bb_mut().terminator = terminator;

        self.use_basic_block(old_block);
        dest
    }

    fn split_critical_edge(
        &mut self,
        pred_id: BasicBlockId,
        succ_id: BasicBlockId,
    ) -> BasicBlockId {
        let needs_split = {
            let pred_bb = self.get_bb(pred_id);
            match &pred_bb.terminator {
                Some(Terminator::CondJump { .. }) => true,
                Some(Terminator::Jump { .. }) => false, // only one successor
                _ => panic!("INTERNAL COMPILER ERROR: Predecessor of Phi ends with Return or None"),
            }
        };

        if !needs_split {
            return pred_id;
        }

        let split_block_id = self.as_fn().new_bb();

        let old_block = self.context.block_id;
        self.use_basic_block(split_block_id);
        self.emit_jmp(succ_id);
        self.seal();

        self.use_basic_block(old_block);

        let pred_bb = self.get_bb_mut(pred_id);

        match &mut pred_bb.terminator {
            Some(Terminator::CondJump {
                true_target,
                false_target,
                ..
            }) => {
                if *true_target == succ_id {
                    *true_target = split_block_id;
                }
                if *false_target == succ_id {
                    *false_target = split_block_id;
                }
            }
            _ => unreachable!("We checked needs_split above"),
        }

        let succ_bb = self.get_bb_mut(succ_id);
        succ_bb.predecessors.remove(&pred_id);

        let split_bb = self.get_bb_mut(split_block_id);
        split_bb.predecessors.insert(pred_id);

        split_block_id
    }

    pub fn new_value_id(&mut self, ty: Type) -> ValueId {
        let value_id = next_value_id();
        let this_block_id = self.context.block_id;

        self.get_fn()
            .value_definitions
            .insert(value_id, this_block_id);

        self.program.value_types.insert(value_id, ty);

        value_id
    }

    pub fn use_basic_block(&mut self, block_id: BasicBlockId) {
        self.context.block_id = block_id;
    }

    pub fn seal(&mut self) {
        if self.bb().sealed {
            return;
        }

        let block_id = self.context.block_id;
        let incomplete = self.incomplete_phis.remove(&block_id).unwrap_or_default();

        for (phi_id, variable, span) in incomplete {
            self.resolve_phi(block_id, phi_id, variable, span);
        }

        self.bb_mut().sealed = true;
    }

    pub fn seal_block(&mut self, block_id: BasicBlockId) {
        let old_block = self.context.block_id;
        self.context.block_id = block_id;
        self.seal();
        self.context.block_id = old_block;
    }
}
