use crate::hir::{
    builders::{
        BasicBlockId, Builder, CheckedFunctionDecl, ExpectBody, InBlock, InFunction,
        InGlobal, InModule,
    },
    types::checked_declaration::CheckedDeclaration,
};

impl<'a> Builder<'a, InBlock> {
    pub fn as_program(&mut self) -> Builder<'_, InGlobal> {
        Builder {
            context: InGlobal,
            program: self.program,
            errors: self.errors,
            current_scope: self.current_scope.clone(),
            own_declarations: self.own_declarations,
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
            own_declarations: self.own_declarations,
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

            own_declarations: self.own_declarations,
        }
    }

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

    pub fn use_basic_block(&mut self, block_id: BasicBlockId) {
        self.context.block_id = block_id;
    }

    pub fn seal(&mut self) {
        if self
            .get_fn_mut()
            .expect_body()
            .get_block_mut(self.context.block_id)
            .sealed
        {
            return;
        }

        for place in incomplete {
            self.read_fact_from_block(block_id, &place);
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
