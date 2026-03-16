use std::collections::{HashMap, HashSet};

use crate::{
    globals::next_block_id,
    mir::{
        builders::{BasicBlock, BasicBlockId, Builder, ExpectBody, InFunction},
        types::checked_declaration::CheckedDeclaration,
    },
};

impl<'a> Builder<'a, InFunction> {
    pub fn new_bb(&mut self) -> BasicBlockId {
        let id = next_block_id();

        let bb = BasicBlock {
            id,
            instructions: vec![],
            sealed: false,
            terminator: None,
            predecessors: HashSet::new(),
            phis: HashMap::new(),
        };

        let decl = self
            .program
            .declarations
            .get_mut(&self.context.func_id)
            .unwrap_or_else(|| {
                panic!(
                    "INTERNAL COMPILER ERROR: Expected function with DeclarationId({}) \
                     to exist.",
                    self.context.func_id.0
                )
            });

        let func = match decl {
            CheckedDeclaration::Function(f) => f,
            _ => panic!("INTERNAL COMPILER ERROR: Declaration is not a function"),
        };

        func.expect_body().blocks.insert(id, bb);

        id
    }
}
