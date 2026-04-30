use crate::hir::builders::{Builder, InGlobal, InModule, Module};

impl<'a> Builder<'a, InModule> {
    pub fn as_program(&mut self) -> Builder<'_, InGlobal> {
        Builder {
            program: self.program,
            errors: self.errors,
            current_scope: self.current_scope.clone(),
            context: InGlobal,
            current_def: self.current_def,
            incomplete_phis: self.incomplete_phis,
            current_memory_def: self.current_memory_def,
            incomplete_memory_phis: self.incomplete_memory_phis,
        }
    }

    pub fn module(&mut self) -> &mut Module {
        self.program.modules.get_mut(&self.context.path).unwrap()
    }
}
