use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

use crate::{
    ast::{
        type_annotation::TypeAnnotation, DeclarationId, GenericDeclarationId,
        IdentifierNode, ModulePath,
    },
    compile::interner::TypeId,
    hir::{
        errors::SemanticError,
        instructions::{BasicBlockId, FunctionCFG},
        types::{
            checked_declaration::{CheckedDeclaration, GenericDeclaration},
            checked_type::SpannedType,
        },
        utils::scope::Scope,
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

pub struct Program {
    pub modules: BTreeMap<ModulePath, Module>,
    pub declarations: BTreeMap<DeclarationId, CheckedDeclaration>,
    pub generic_declarations: BTreeMap<GenericDeclarationId, GenericDeclaration>,

    pub entry_path: Option<ModulePath>,
    pub target_ptr_size: usize,
    pub target_ptr_align: usize,
    pub foreign_links: HashSet<PathBuf>,

    pub monomorphizations: BTreeMap<(GenericDeclarationId, Vec<TypeId>), DeclarationId>,
}

pub struct Module {
    pub path: ModulePath,
    pub root_scope: Scope,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub identifier: IdentifierNode,
    pub ty: TypeAnnotation,
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

pub trait BuilderContext {}
pub struct Builder<'a, C: BuilderContext> {
    pub context: C,
    pub program: &'a mut Program,

    pub errors: &'a mut Vec<SemanticError>,
    pub current_scope: Scope,

    // Tracks declarations created by this specific builder
    pub own_declarations: &'a mut HashSet<DeclarationId>,
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
