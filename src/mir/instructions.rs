use crate::mir::builders::{BasicBlockId, ValueId};

#[derive(Clone, Debug)]
pub enum BinaryInstr {
    IAdd {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    ISub {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    IMul {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    SDiv {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    UDiv {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    SRem {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    URem {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FRem {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FAdd {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FSub {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FMul {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FDiv {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
}

#[derive(Clone, Debug)]
pub enum CompInstr {
    IEq {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    INeq {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    SLt {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    SLte {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    SGt {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    SGte {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    ULt {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    ULte {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    UGt {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    UGte {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FEq {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FNeq {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FLt {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FLte {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FGt {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    FGte {
        dest: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
}

#[derive(Clone, Debug)]
pub enum UnaryInstr {
    INeg { dest: ValueId, src: ValueId },
    FNeg { dest: ValueId, src: ValueId },
    BNot { dest: ValueId, src: ValueId },
}

#[derive(Clone, Debug)]
pub enum CastInstr {
    SIToF { dest: ValueId, src: ValueId },
    UIToF { dest: ValueId, src: ValueId },
    FToSI { dest: ValueId, src: ValueId },
    FToUI { dest: ValueId, src: ValueId },
    FExt { dest: ValueId, src: ValueId },
    FTrunc { dest: ValueId, src: ValueId },
    Trunc { dest: ValueId, src: ValueId },
    SExt { dest: ValueId, src: ValueId },
    ZExt { dest: ValueId, src: ValueId },
    BitCast { dest: ValueId, src: ValueId },
}

#[derive(Clone, Debug)]
pub enum MemoryInstr {
    StackAlloc {
        dest: ValueId,
        count: usize,
    },
    HeapAlloc {
        dest: ValueId,
        count: ValueId,
    },
    HeapFree {
        ptr: ValueId,
    },
    Store {
        ptr: ValueId,
        value: ValueId,
    },
    Load {
        dest: ValueId,
        ptr: ValueId,
    },
    MemCopy {
        dest: ValueId,
        src: ValueId,
    },
    GetFieldPtr {
        dest: ValueId,
        base_ptr: ValueId,
        field_index: usize,
    },
    PtrOffset {
        dest: ValueId,
        base_ptr: ValueId,
        index: ValueId,
    },
}

#[derive(Clone, Debug)]
pub struct CallInstr {
    pub dest: ValueId,
    pub func: ValueId,
    pub args: Vec<ValueId>,
}

#[derive(Clone, Debug)]
pub struct SelectInstr {
    pub dest: ValueId,
    pub cond: ValueId,
    pub true_val: ValueId,
    pub false_val: ValueId,
}

#[derive(Clone, Debug)]
pub enum Instruction {
    Cast(CastInstr),
    Unary(UnaryInstr),
    Binary(BinaryInstr),
    Comp(CompInstr),
    Memory(MemoryInstr),
    Call(CallInstr),
    Select(SelectInstr),
}

#[derive(Clone, Debug)]
pub enum Terminator {
    Jump {
        target: BasicBlockId,
    },
    CondJump {
        condition: ValueId,
        true_target: BasicBlockId,
        false_target: BasicBlockId,
    },
    Return {
        value: ValueId,
    },
    Panic {
        message: Option<ValueId>,
    },
}
