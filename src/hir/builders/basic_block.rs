use std::collections::BTreeSet;

use crate::{
    ast::{
        decl::{Declaration, FnDecl},
        DeclarationId, Span,
    },
    hir::{
        builders::{BasicBlockId, Builder, InBlock, InFunction, InModule},
        instructions::{
            BasicBlock, FunctionCFG, InstrDefinition, InstrId, InstructionKind, MemoryId,
            MemoryPhiSource, PhiInstr, PhiSource,
        },
    },
};

impl<'a> Builder<'a, InBlock> {
    pub fn as_module(&mut self) -> Builder<'_, InModule> {
        Builder {
            context: InModule {
                path: self.context.path.clone(),
            },
            program: self.program,
            errors: self.errors,
            current_scope: self.current_scope.clone(),
            current_def: self.current_def,
            incomplete_phis: self.incomplete_phis,
            current_memory_def: self.current_memory_def,
            incomplete_memory_phis: self.incomplete_memory_phis,
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
            current_def: self.current_def,
            incomplete_phis: self.incomplete_phis,
            current_memory_def: self.current_memory_def,
            incomplete_memory_phis: self.incomplete_memory_phis,
        }
    }

    pub fn new_block(&mut self) -> BasicBlockId {
        self.cfg_mut().new_block()
    }

    pub fn cfg(&self) -> &FunctionCFG {
        self.program.cfgs.get(&self.context.func_id).unwrap()
    }

    pub fn cfg_mut(&mut self) -> &mut FunctionCFG {
        self.program.cfgs.get_mut(&self.context.func_id).unwrap()
    }

    pub fn bb(&self) -> &BasicBlock {
        let block = self.context.block_id;
        self.get_bb(block)
    }

    pub fn bb_mut(&mut self) -> &mut BasicBlock {
        let block = self.context.block_id;
        self.get_bb_mut(block)
    }

    pub fn get_bb(&self, block: BasicBlockId) -> &BasicBlock {
        self.cfg().get_block(block)
    }

    pub fn get_bb_mut(&mut self, block: BasicBlockId) -> &mut BasicBlock {
        self.cfg_mut().get_block_mut(block)
    }

    pub fn get_fn(&self) -> &FnDecl {
        let func_id = self.context.func_id;

        match self.program.declarations.get(&func_id).unwrap() {
            Declaration::Fn(f) => f,
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        }
    }

    pub fn get_fn_mut(&mut self) -> &mut FnDecl {
        let func_id = self.context.func_id;

        match self.program.declarations.get_mut(&func_id).unwrap() {
            Declaration::Fn(f) => f,
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        }
    }

    pub fn get_instr_span(&self, id: InstrId) -> &Span {
        &self.cfg().get_instr(id).span
    }

    pub fn use_basic_block(&mut self, block_id: BasicBlockId) {
        self.context.block_id = block_id;
    }

    pub fn seal(&mut self) {
        let block_id = self.context.block_id;
        if self.bb().sealed {
            return;
        }

        let incomplete_data = self.incomplete_phis.remove(&block_id).unwrap_or_default();
        for (var_decl, phi_instr_id) in incomplete_data {
            self.add_phi_operands(block_id, var_decl, phi_instr_id);
        }

        if let Some(mem_phi_id) = self.incomplete_memory_phis.remove(&block_id) {
            self.add_memory_phi_operands(block_id, mem_phi_id);
        }

        self.bb_mut().sealed = true;
    }

    pub fn seal_block(&mut self, block_id: BasicBlockId) {
        let old_block = self.context.block_id;
        self.context.block_id = block_id;
        self.seal();
        self.context.block_id = old_block;
    }

    fn add_phi_operands(
        &mut self,
        block_id: BasicBlockId,
        var_decl: DeclarationId,
        phi_instr_id: InstrId,
    ) {
        let predecessors = self.get_bb(block_id).predecessors.clone();
        let mut sources = Vec::new();

        for pred in predecessors {
            let val = self.read_variable(pred, var_decl);
            sources.push(PhiSource {
                block: pred,
                value: val,
            });
        }

        let instr_def = self.cfg_mut().get_instr_mut(phi_instr_id);
        if let InstructionKind::Phi(phi) = &mut instr_def.kind {
            phi.sources = sources;
        } else {
            panic!("INTERNAL COMPILER ERROR: Expected Phi instruction");
        }
    }

    pub fn write_variable(
        &mut self,
        block: BasicBlockId,
        var: DeclarationId,
        value: InstrId,
    ) {
        self.current_def
            .entry(block)
            .or_default()
            .insert(var, value);
    }

    pub fn read_variable(&mut self, block: BasicBlockId, var: DeclarationId) -> InstrId {
        if let Some(&val) = self.current_def.get(&block).and_then(|defs| defs.get(&var)) {
            return val;
        }
        self.read_variable_recursive(block, var)
    }

    pub fn read_variable_recursive(
        &mut self,
        block: BasicBlockId,
        var: DeclarationId,
    ) -> InstrId {
        let is_sealed = self.get_bb(block).sealed;

        if !is_sealed {
            let phi_id = self.emit_incomplete_phi(block);
            self.incomplete_phis
                .entry(block)
                .or_default()
                .insert(var, phi_id);
            self.write_variable(block, var, phi_id);

            return phi_id;
        }

        let preds = self.get_bb(block).predecessors.clone();

        if preds.len() == 1 {
            let pred = preds.into_iter().next().unwrap();
            let val = self.read_variable(pred, var);
            self.write_variable(block, var, val);

            return val;
        }

        let phi_id = self.emit_incomplete_phi(block);
        self.write_variable(block, var, phi_id);

        self.add_phi_operands(block, var, phi_id);

        phi_id
    }

    fn emit_incomplete_phi(&mut self, block: BasicBlockId) -> InstrId {
        let cfg = self.cfg_mut();
        let id = InstrId(cfg.instructions.len());

        let instr = InstrDefinition {
            kind: InstructionKind::Phi(PhiInstr { sources: vec![] }),
            block,
            span: Span::default(),
        };

        cfg.instructions.push(instr);

        let mut insert_idx = 0;
        {
            let bb = cfg.get_block(block);
            for &existing_id in &bb.instructions {
                if matches!(
                    cfg.instructions[existing_id.0].kind,
                    InstructionKind::Phi(_)
                ) {
                    insert_idx += 1;
                } else {
                    break;
                }
            }
        }

        cfg.get_block_mut(block).instructions.insert(insert_idx, id);

        id
    }

    pub fn write_memory(&mut self, block: BasicBlockId, mem: MemoryId) {
        self.current_memory_def.insert(block, mem);
    }

    pub fn read_memory(&mut self, block: BasicBlockId) -> MemoryId {
        if let Some(&mem) = self.current_memory_def.get(&block) {
            return mem;
        }
        self.read_memory_recursive(block)
    }

    fn read_memory_recursive(&mut self, block: BasicBlockId) -> MemoryId {
        let is_sealed = self.get_bb(block).sealed;

        if !is_sealed {
            let phi_id = self.cfg_mut().new_memory_id();
            self.incomplete_memory_phis.insert(block, phi_id);
            self.write_memory(block, phi_id);
            return phi_id;
        }

        let preds = self.get_bb(block).predecessors.clone();

        if preds.len() == 1 {
            let pred = preds.into_iter().next().unwrap();
            let mem = self.read_memory(pred);
            self.write_memory(block, mem);
            return mem;
        }

        let phi_id = self.cfg_mut().new_memory_id();
        self.write_memory(block, phi_id);
        self.add_memory_phi_operands(block, phi_id);

        phi_id
    }

    fn add_memory_phi_operands(&mut self, block: BasicBlockId, phi_id: MemoryId) {
        let preds = self.get_bb(block).predecessors.clone();
        let mut sources = BTreeSet::new();

        for pred in preds {
            let mem = self.read_memory(pred);
            sources.insert(MemoryPhiSource {
                block: pred,
                memory: mem,
            });
        }

        self.cfg_mut()
            .get_block_mut(block)
            .memory_phis
            .insert(phi_id, sources);
    }
}
