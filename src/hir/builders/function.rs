use std::collections::HashSet;

use crate::hir::{
    builders::{
        BasicBlock, BasicBlockId, Builder, CheckedFunctionDecl, ExpectBody, InFunction,
    },
    types::checked_declaration::CheckedDeclaration,
};

impl<'a> Builder<'a, InFunction> {
    pub fn get_fn(&self) -> &CheckedFunctionDecl {
        let func_id = self.context.func_id;

        match self.program.declarations.get(&func_id).unwrap() {
            CheckedDeclaration::Function(f) => f,
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        }
    }

    pub fn get_fn_mut(&mut self) -> &mut CheckedFunctionDecl {
        let func_id = self.context.func_id;

        match self.program.declarations.get_mut(&func_id).unwrap() {
            CheckedDeclaration::Function(f) => f,
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        }
    }

    pub fn new_bb(&mut self) -> BasicBlockId {
        let func = self.get_fn_mut().expect_body();

        let id_num = func.next_block_id;
        func.next_block_id += 1;
        let id = BasicBlockId(id_num);

        let bb = BasicBlock {
            id,
            instructions: vec![],
            sealed: false,
            terminator: None,
            predecessors: HashSet::new(),
        };

        func.blocks.insert(id, bb);

        id
    }
}
