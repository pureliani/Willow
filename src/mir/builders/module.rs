use crate::mir::builders::{Builder, InGlobal, InModule, Module};

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
            ptg: self.ptg,
            aliases: self.aliases,
            types: self.types,
        }
    }

    pub fn module(&mut self) -> &mut Module {
        self.program.modules.get_mut(&self.context.path).unwrap()
    }
}
