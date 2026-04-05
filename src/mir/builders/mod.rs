use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
};

use crate::{
    ast::{DeclarationId, IdentifierNode, ModulePath},
    compile::interner::{TypeId, TypeInterner},
    mir::{
        errors::SemanticError,
        instructions::{Instruction, Terminator},
        types::{
            checked_declaration::{CheckedDeclaration, FunctionEffects},
            checked_type::SpannedType,
        },
        utils::{facts::FactSet, place::Place, points_to::PointsToGraph, scope::Scope},
    },
};

pub mod basic_block;
pub mod binary;
pub mod cast;
pub mod comp;
pub mod r#const;
pub mod control_flow;
pub mod function;
pub mod memory;
pub mod module;
pub mod program;
pub mod std_lib;
pub mod unary;
pub mod union;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BasicBlockId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoopJumpTargets {
    pub on_break: BasicBlockId,
    pub on_continue: BasicBlockId,
}

pub struct Program {
    pub modules: BTreeMap<ModulePath, Module>,
    pub value_types: BTreeMap<ValueId, TypeId>,
    pub declarations: BTreeMap<DeclarationId, CheckedDeclaration>,
    pub entry_path: Option<ModulePath>,

    pub target_ptr_size: usize,
    pub target_ptr_align: usize,

    pub foreign_links: HashSet<PathBuf>,
}

pub struct Module {
    pub path: ModulePath,
    pub root_scope: Scope,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub identifier: IdentifierNode,
    pub ty: SpannedType,
    pub decl_id: Option<DeclarationId>,
    pub value_id: Option<ValueId>,
}

#[derive(Debug, Clone)]
pub struct FunctionCFG {
    pub entry_block: BasicBlockId,
    pub blocks: BTreeMap<BasicBlockId, BasicBlock>,

    pub value_definitions: BTreeMap<ValueId, BasicBlockId>,
    pub ptg: PointsToGraph,
    pub effects: FunctionEffects,
}

#[derive(Debug, Clone)]
pub enum FunctionBodyKind {
    Internal(FunctionCFG),
    External,
    NotBuilt,
}

#[derive(Debug, Clone)]
pub struct CheckedFunctionDecl {
    pub id: DeclarationId,
    pub identifier: IdentifierNode,
    pub params: Vec<FunctionParam>,
    pub return_type: SpannedType,
    pub is_exported: bool,
    pub body: FunctionBodyKind,
}

pub trait ExpectBody {
    type Output;
    fn expect_body(self) -> Self::Output;
}

impl<'a> ExpectBody for &'a CheckedFunctionDecl {
    type Output = &'a FunctionCFG;
    fn expect_body(self) -> Self::Output {
        match &self.body {
            FunctionBodyKind::Internal(cfg) => cfg,
            _ => panic!("INTERNAL COMPILER ERROR: Expected internal function"),
        }
    }
}

impl<'a> ExpectBody for &'a mut CheckedFunctionDecl {
    type Output = &'a mut FunctionCFG;
    fn expect_body(self) -> Self::Output {
        match &mut self.body {
            FunctionBodyKind::Internal(cfg) => cfg,
            _ => panic!("INTERNAL COMPILER ERROR: Expected internal function"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhiSource {
    pub from: BasicBlockId,
    pub value: ValueId,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BasicBlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
    pub predecessors: HashSet<BasicBlockId>,
    pub sealed: bool,
}

#[derive(Debug, Clone)]
pub struct ConditionFact {
    pub place: Place,
    pub on_true: FactSet,
    pub on_false: FactSet,
}

pub trait BuilderContext {}
pub struct Builder<'a, C: BuilderContext> {
    pub context: C,
    pub program: &'a mut Program,
    pub types: &'a TypeInterner,

    pub errors: &'a mut Vec<SemanticError>,
    pub current_scope: Scope,

    pub current_facts: &'a mut HashMap<BasicBlockId, HashMap<Place, FactSet>>,
    pub incomplete_fact_merges: &'a mut HashMap<BasicBlockId, Vec<Place>>,
    pub condition_facts: &'a mut HashMap<ValueId, Vec<ConditionFact>>,

    pub aliases: &'a mut HashMap<DeclarationId, Place>,
    pub ptg: &'a mut PointsToGraph,
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
