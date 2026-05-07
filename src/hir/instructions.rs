use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ast::{type_annotation::TypeAnnotation, DeclarationId, IdentifierNode, Span},
    compile::interner::StringId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemoryId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AllocationId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AccessKind {
    Field(usize),
    Index(InstrId),
    Deref,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    pub base: AllocationId,
    pub projections: Vec<AccessKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BasicBlockId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstrId(pub usize);

#[derive(Clone, Debug)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Clone, Debug)]
pub struct BinaryInstr {
    pub lhs: InstrId,
    pub rhs: InstrId,
    pub op: BinaryOpKind,
}

#[derive(Clone, Debug)]
pub enum UnaryOpKind {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub struct UnaryInstr {
    pub value: InstrId,
    pub op: UnaryOpKind,
}

#[derive(Clone, Debug)]
pub struct CastInstr {
    pub src: InstrId,
    pub target: TypeAnnotation,
}

#[derive(Clone, Debug)]
pub enum MemoryInstr {
    StackAlloc {
        id: AllocationId,
        ty: TypeAnnotation,
        count: usize,
    },
    HeapAlloc {
        id: AllocationId,
        ty: TypeAnnotation,
        count: InstrId,
    },
    HeapFree {
        ptr: InstrId,
        memory_in: MemoryId,
        memory_out: MemoryId,
    },
    MemCopy {
        from: InstrId,
        to: InstrId,
        memory_in: MemoryId,
        memory_out: MemoryId,
    },
    ProjectPtr {
        base_ptr: InstrId,
        access: AccessKind,
    },
    Load {
        ptr: InstrId,
        memory_in: MemoryId,
    },
    Store {
        ptr: InstrId,
        value: InstrId,
        memory_in: MemoryId,
        memory_out: MemoryId,
    },
}

#[derive(Clone, Debug)]
pub struct CallInstr {
    pub func: InstrId,
    pub args: Vec<InstrId>,
    pub memory_in: MemoryId,
    pub memory_out: MemoryId,
}

#[derive(Clone, Debug)]
pub struct SelectInstr {
    pub cond: InstrId,
    pub true_val: InstrId,
    pub false_val: InstrId,
}

#[derive(Clone, Debug)]
pub struct PhiSource {
    pub block: BasicBlockId,
    pub value: InstrId,
}

#[derive(Clone, Debug)]
pub struct PhiInstr {
    pub sources: Vec<PhiSource>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryPhiSource {
    pub block: BasicBlockId,
    pub memory: MemoryId,
}

#[derive(Clone, Debug)]
pub enum MakeLiteralKind {
    String(StringId),
    Fn(DeclarationId),
    Null,
    Void,
    Unknown,
    Never,
}

#[derive(Clone, Debug)]
pub struct StructInitInstr {
    pub fields: Vec<(IdentifierNode, InstrId)>,
    pub by_value: bool,
}

#[derive(Clone, Debug)]
pub struct GenericApplyInstr {
    pub func: InstrId,
    pub type_args: Vec<TypeAnnotation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuiltinFunction {
    StringConcat,
}

#[derive(Clone, Debug)]
pub struct CallBuiltinInstr {
    pub builtin: BuiltinFunction,
    pub args: Vec<InstrId>,
    pub memory_in: MemoryId,
    pub memory_out: MemoryId,
}

#[derive(Clone, Debug)]
pub enum InstructionKind {
    Unary(UnaryInstr),
    Binary(BinaryInstr),
    Cast(CastInstr),
    Memory(MemoryInstr),
    Call(CallInstr),
    Select(SelectInstr),
    Phi(PhiInstr),
    Param(usize),
    StructInit(StructInitInstr),
    GenericApply(GenericApplyInstr),
    CallBuiltin(CallBuiltinInstr),
}

#[derive(Clone, Debug)]
pub enum Terminator {
    Jump {
        target: BasicBlockId,
    },
    CondJump {
        condition: InstrId,
        true_target: BasicBlockId,
        false_target: BasicBlockId,
    },
    Return {
        value: InstrId,
    },
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BasicBlockId,
    pub memory_phis: BTreeMap<MemoryId, BTreeSet<MemoryPhiSource>>,
    pub instructions: Vec<InstrId>,
    pub terminator: Option<Terminator>,
    pub predecessors: BTreeSet<BasicBlockId>,
    pub sealed: bool,
}

#[derive(Debug, Clone)]
pub struct InstrDefinition {
    pub kind: InstructionKind,
    pub block: BasicBlockId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionCFG {
    pub entry_block: BasicBlockId,
    pub blocks: BTreeMap<BasicBlockId, BasicBlock>,
    pub instructions: Vec<InstrDefinition>,

    next_block_id: usize,
    next_memory_id: usize,
}

impl FunctionCFG {
    pub fn new() -> Self {
        let mut cfg = Self {
            entry_block: BasicBlockId(0),
            blocks: BTreeMap::new(),
            instructions: Vec::new(),
            next_block_id: 0,
            next_memory_id: 1,
        };

        cfg.entry_block = cfg.new_block();
        cfg
    }

    pub fn new_block(&mut self) -> BasicBlockId {
        let id_num = self.next_block_id;
        self.next_block_id += 1;
        let id = BasicBlockId(id_num);

        let block = BasicBlock {
            id,
            instructions: vec![],
            sealed: false,
            terminator: None,
            predecessors: BTreeSet::new(),
            memory_phis: BTreeMap::new(),
        };

        self.blocks.insert(id, block);

        id
    }

    pub fn new_memory_id(&mut self) -> MemoryId {
        let id = MemoryId(self.next_memory_id);
        self.next_memory_id += 1;
        id
    }

    pub fn push_instruction(&mut self, instr: InstrDefinition) -> InstrId {
        let block_id = instr.block;

        self.check_no_terminator(block_id);

        if matches!(instr.kind, InstructionKind::Phi(_)) {
            self.validate_phi_push(block_id);
        }

        let id = InstrId(self.instructions.len());

        self.instructions.push(instr);
        self.get_block_mut(block_id).instructions.push(id);

        id
    }

    pub fn get_block_mut(&mut self, block_id: BasicBlockId) -> &mut BasicBlock {
        self.blocks.get_mut(&block_id).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: Expected block `{}` to exist",
                block_id.0
            )
        })
    }

    pub fn get_block(&self, block_id: BasicBlockId) -> &BasicBlock {
        self.blocks.get(&block_id).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: Expected block `{}` to exist",
                block_id.0
            )
        })
    }

    pub fn get_instr_mut(&mut self, instr_id: InstrId) -> &mut InstrDefinition {
        self.instructions.get_mut(instr_id.0).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: Expected instruction `{}` to exist",
                instr_id.0
            )
        })
    }

    pub fn get_instr(&self, instr_id: InstrId) -> &InstrDefinition {
        self.instructions.get(instr_id.0).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: Expected instruction `{}` to exist",
                instr_id.0
            )
        })
    }

    fn validate_phi_push(&self, block_id: BasicBlockId) {
        let block = self.get_block(block_id);

        for &instr_id in &block.instructions {
            let instr = self.get_instr(instr_id);
            if !matches!(instr.kind, InstructionKind::Phi(_)) {
                panic!("INTERNAL COMPILER ERROR: Phi instructions must be emitted first");
            }
        }
    }

    pub fn check_no_terminator(&self, block_id: BasicBlockId) {
        let block = self.get_block(block_id);

        if block.terminator.is_some() {
            panic!(
                "INTERNAL COMPILER ERROR: Tried to add an instruction to a basic block with id `{}` that has already been terminated",
                block.id.0
            );
        }
    }
}
