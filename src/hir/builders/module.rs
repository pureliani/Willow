use crate::hir::builders::{Builder, InGlobal, InModule, Module};

impl<'a> Builder<'a, InModule> {
    pub fn as_program(&mut self) -> Builder<'_, InGlobal> {
        Builder {
            program: self.program,
            errors: self.errors,
            current_scope: self.current_scope.clone(),
            context: InGlobal,
            condition_facts: self.condition_facts,
            current_facts: self.current_facts,
            incomplete_fact_merges: self.incomplete_fact_merges,
            aliases: self.aliases,
            types: self.types,
            own_declarations: self.own_declarations,
        }
    }

    pub fn module(&mut self) -> &mut Module {
        self.program.modules.get_mut(&self.context.path).unwrap()
    }
}
