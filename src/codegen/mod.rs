pub mod functions;
pub mod instructions;
pub mod types;

use std::collections::HashMap;
use std::path::Path;

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    targets::TargetMachine,
    values::{BasicValueEnum, FunctionValue},
};

use crate::{
    ast::DeclarationId,
    compile::interner::TypeInterner,
    mir::builders::{BasicBlockId, Program, ValueId},
};

pub struct CodeGenerator<'ctx, 'a> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub target_machine: TargetMachine,

    pub program: &'a Program,
    pub type_interner: &'a TypeInterner,

    pub values: HashMap<ValueId, BasicValueEnum<'ctx>>,
    pub functions: HashMap<DeclarationId, FunctionValue<'ctx>>,
    pub blocks: HashMap<BasicBlockId, BasicBlock<'ctx>>,
}

impl<'ctx, 'a> CodeGenerator<'ctx, 'a> {
    pub fn new(
        context: &'ctx Context,
        program: &'a Program,
        interner: &'a TypeInterner,
        target_machine: TargetMachine,
    ) -> Self {
        let module = context.create_module("willow_module");
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            target_machine,
            program,
            type_interner: interner,
            values: HashMap::new(),
            functions: HashMap::new(),
            blocks: HashMap::new(),
        }
    }

    pub fn generate_ir(&mut self) {
        self.declare_functions();

        self.define_functions();

        if let Err(err) = self.module.verify() {
            eprintln!("LLVM Verification Failed:\n{}", err.to_string());
        }
    }

    pub fn dump_ir(&self, path: &Path) {
        self.module.print_to_file(path).unwrap();
    }

    pub fn emit_object_file(&self, path: &Path) {
        self.target_machine
            .write_to_file(&self.module, inkwell::targets::FileType::Object, path)
            .unwrap();
    }
}
