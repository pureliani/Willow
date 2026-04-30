use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

use crate::{
    ast::{decl::Declaration, DeclarationId, ModulePath},
    hir::{
        errors::SemanticError,
        instructions::{BasicBlockId, FunctionCFG, InstrId, MemoryId},
        utils::scope::Scope,
    },
};

pub mod basic_block;
pub mod r#const;
pub mod control_flow;
pub mod memory;
pub mod module;
pub mod program;
pub mod std_lib;

pub struct Program {
    pub modules: BTreeMap<ModulePath, Module>,
    pub declarations: BTreeMap<DeclarationId, Declaration>,
    pub cfgs: BTreeMap<DeclarationId, FunctionCFG>,
    pub entry_path: Option<ModulePath>,
    pub target_ptr_size: usize,
    pub target_ptr_align: usize,
    pub foreign_links: HashSet<PathBuf>,
}

pub struct Module {
    pub path: ModulePath,
    pub root_scope: Scope,
}

pub trait BuilderContext {}
pub struct Builder<'a, C: BuilderContext> {
    pub context: C,
    pub program: &'a mut Program,

    pub errors: &'a mut Vec<SemanticError>,
    pub current_scope: Scope,

    pub current_def: &'a mut BTreeMap<BasicBlockId, BTreeMap<DeclarationId, InstrId>>,
    pub incomplete_phis: &'a mut BTreeMap<BasicBlockId, BTreeMap<DeclarationId, InstrId>>,

    pub current_memory_def: &'a mut BTreeMap<BasicBlockId, MemoryId>,
    pub incomplete_memory_phis: &'a mut BTreeMap<BasicBlockId, MemoryId>,
}

pub struct InGlobal;
impl BuilderContext for InGlobal {}

pub struct InModule {
    pub path: ModulePath,
}
impl BuilderContext for InModule {}

pub struct InFunction {
    pub path: ModulePath,
    pub func_id: DeclarationId,
}
impl BuilderContext for InFunction {}

pub struct InBlock {
    pub path: ModulePath,
    pub func_id: DeclarationId,
    pub block_id: BasicBlockId,
}
impl BuilderContext for InBlock {}
