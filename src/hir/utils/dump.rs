use crate::{
    compile::interner::TypeInterner,
    globals::STRING_INTERNER,
    hir::{
        builders::Program,
        instructions::{
            BinaryInstr, CallInstr, CastInstr, InstructionKind, MemoryInstr, SelectInstr,
            Terminator, UnaryInstr,
        },
        types::checked_type::Type,
    },
};
use std::fmt::Write;

fn get_vt(cfg: &FunctionCFG, vid: &ValueId, interner: &TypeInterner) -> String {
    interner.to_string(cfg.values[vid].ty)
}

pub fn dump_program(program: &Program, interner: &TypeInterner) {
    let mut out = String::new();
    writeln!(out, "========== HIR DUMP START ==========").unwrap();
    for (_, decl) in program.declarations.iter() {
        if let CheckedDeclaration::Function(f) = decl {
            dump_function(f, interner, &mut out);
        }
    }
    writeln!(out, "====================================").unwrap();
    println!("{}", out);
}

fn dump_function(f: &CheckedFunctionDecl, interner: &TypeInterner, out: &mut String) {
    let fn_name = STRING_INTERNER.resolve(f.identifier.name);
    let return_type = interner.to_string(f.return_type.id);
    writeln!(out, "fn {fn_name} -> {return_type}:").unwrap();

    let block_ids = if let FunctionBodyKind::Internal(cfg) = &f.body {
        cfg.blocks.keys().cloned().collect()
    } else {
        vec![]
    };

    for bid in block_ids {
        dump_block(&bid, f, interner, out);
    }
}

pub fn dump_block(
    block_id: &BasicBlockId,
    f: &CheckedFunctionDecl,
    interner: &TypeInterner,
    out: &mut String,
) {
    let cfg = f.expect_body();
    let bb = cfg.blocks.get(block_id).unwrap();
    writeln!(out, "  block_{}:", bb.id.0).unwrap();

    writeln!(out, "    predecessors {{ ").unwrap();
    for p in &bb.predecessors {
        writeln!(out, "      block_{}", p.0).unwrap();
    }
    writeln!(out, "    }} ").unwrap();

    writeln!(out).unwrap();

    dump_instructions(&bb.instructions, cfg, interner, out);

    if let Some(term) = bb.terminator.clone() {
        match term {
            Terminator::Jump { target } => {
                writeln!(out, "    jmp block_{}", target.0).unwrap();
            }
            Terminator::CondJump {
                condition,
                true_target,
                false_target,
            } => {
                writeln!(
                    out,
                    "    cond_jmp v{} ? block_{} : block_{}\n",
                    condition.0, true_target.0, false_target.0
                )
                .unwrap();
            }
            Terminator::Return { value } => {
                writeln!(out, "    ret v{}\n", value.0).unwrap();
            }
            Terminator::Unreachable => {
                writeln!(out, "    unreachable\n").unwrap();
            }
        }
    }
}

pub fn dump_instructions(
    instrs: &[InstructionKind],
    cfg: &FunctionCFG,
    interner: &TypeInterner,
    out: &mut String,
) {
    let get_binary_sign = |instr: &BinaryInstr| match instr {
        BinaryInstr::IAdd { .. } | BinaryInstr::FAdd { .. } => "+",
        BinaryInstr::ISub { .. } | BinaryInstr::FSub { .. } => "-",
        BinaryInstr::IMul { .. } | BinaryInstr::FMul { .. } => "*",
        BinaryInstr::SDiv { .. }
        | BinaryInstr::UDiv { .. }
        | BinaryInstr::FDiv { .. } => "/",
        BinaryInstr::SRem { .. }
        | BinaryInstr::URem { .. }
        | BinaryInstr::FRem { .. } => "%",
    };

    let get_comp_sign = |instr: &CompInstr| match instr {
        CompInstr::IEq { .. } | CompInstr::FEq { .. } => "==",
        CompInstr::INeq { .. } | CompInstr::FNeq { .. } => "!=",
        CompInstr::SLt { .. } | CompInstr::ULt { .. } | CompInstr::FLt { .. } => "<",
        CompInstr::SLte { .. } | CompInstr::ULte { .. } | CompInstr::FLte { .. } => "<=",
        CompInstr::SGt { .. } | CompInstr::UGt { .. } | CompInstr::FGt { .. } => ">",
        CompInstr::SGte { .. } | CompInstr::UGte { .. } | CompInstr::FGte { .. } => ">=",
    };

    let get_cast_name = |instr: &CastInstr| match instr {
        CastInstr::SIToF { .. } => "SIToF",
        CastInstr::FToSI { .. } => "FToSI",
        CastInstr::FExt { .. } => "FExt",
        CastInstr::FTrunc { .. } => "FTrunc",
        CastInstr::Trunc { .. } => "Trunc",
        CastInstr::SExt { .. } => "SExt",
        CastInstr::ZExt { .. } => "ZExt",
        CastInstr::BitCast { .. } => "BitCast",
        CastInstr::UIToF { .. } => "UIToF",
        CastInstr::FToUI { .. } => "FToUI",
    };

    for instruction in instrs {
        write!(out, "    ").unwrap();
        match instruction {
            InstructionKind::Unary(kind) => match kind {
                UnaryInstr::INeg { dest, src } | UnaryInstr::FNeg { dest, src } => {
                    writeln!(
                        out,
                        "v{}: {} = -v{};",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        src.0
                    )
                    .unwrap();
                }
                UnaryInstr::BNot { dest, src } => {
                    writeln!(
                        out,
                        "v{}: {} = !v{};",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        src.0
                    )
                    .unwrap();
                }
            },
            InstructionKind::Binary(kind) => match kind {
                BinaryInstr::IAdd { dest, lhs, rhs }
                | BinaryInstr::ISub { dest, lhs, rhs }
                | BinaryInstr::IMul { dest, lhs, rhs }
                | BinaryInstr::SDiv { dest, lhs, rhs }
                | BinaryInstr::UDiv { dest, lhs, rhs }
                | BinaryInstr::SRem { dest, lhs, rhs }
                | BinaryInstr::URem { dest, lhs, rhs }
                | BinaryInstr::FRem { dest, lhs, rhs }
                | BinaryInstr::FAdd { dest, lhs, rhs }
                | BinaryInstr::FSub { dest, lhs, rhs }
                | BinaryInstr::FMul { dest, lhs, rhs }
                | BinaryInstr::FDiv { dest, lhs, rhs } => {
                    writeln!(
                        out,
                        "v{}: {} = v{} {} v{};",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        lhs.0,
                        get_binary_sign(kind),
                        rhs.0
                    )
                    .unwrap();
                }
            },
            InstructionKind::Comp(kind) => match kind {
                CompInstr::IEq { dest, lhs, rhs }
                | CompInstr::INeq { dest, lhs, rhs }
                | CompInstr::SLt { dest, lhs, rhs }
                | CompInstr::SLte { dest, lhs, rhs }
                | CompInstr::SGt { dest, lhs, rhs }
                | CompInstr::SGte { dest, lhs, rhs }
                | CompInstr::ULt { dest, lhs, rhs }
                | CompInstr::ULte { dest, lhs, rhs }
                | CompInstr::UGt { dest, lhs, rhs }
                | CompInstr::UGte { dest, lhs, rhs }
                | CompInstr::FEq { dest, lhs, rhs }
                | CompInstr::FNeq { dest, lhs, rhs }
                | CompInstr::FLt { dest, lhs, rhs }
                | CompInstr::FLte { dest, lhs, rhs }
                | CompInstr::FGt { dest, lhs, rhs }
                | CompInstr::FGte { dest, lhs, rhs } => {
                    writeln!(
                        out,
                        "v{}: {} = v{} {} v{};",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        lhs.0,
                        get_comp_sign(kind),
                        rhs.0
                    )
                    .unwrap();
                }
            },
            InstructionKind::Select(SelectInstr {
                dest,
                cond,
                true_val,
                false_val,
            }) => {
                writeln!(
                    out,
                    "v{}: {} = v{} ? v{} : v{};",
                    dest.0,
                    get_vt(cfg, dest, interner),
                    cond.0,
                    true_val.0,
                    false_val.0
                )
                .unwrap();
            }
            InstructionKind::Call(CallInstr { dest, func, args }) => {
                let args = args
                    .iter()
                    .map(|a| format!("v{}", a.0))
                    .collect::<Vec<String>>()
                    .join(", ");

                writeln!(
                    out,
                    "v{}: {} = call v{}({});",
                    dest.0,
                    get_vt(cfg, dest, interner),
                    func.0,
                    args
                )
                .unwrap();
            }

            InstructionKind::Memory(kind) => match kind {
                MemoryInstr::StackAlloc { dest, count } => {
                    let inner_ty = match interner.resolve(cfg.values[dest].ty) {
                        Type::Pointer(to) => interner.to_string(to),
                        _ => "unknown".to_string(),
                    };
                    writeln!(
                        out,
                        "v{}: {} = stackAlloc({} x {});",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        count,
                        inner_ty
                    )
                    .unwrap();
                }
                MemoryInstr::HeapAlloc { dest, count } => {
                    let inner_ty = match interner.resolve(cfg.values[dest].ty) {
                        Type::Pointer(to) => interner.to_string(to),
                        _ => "unknown".to_string(),
                    };
                    writeln!(
                        out,
                        "v{}: {} = heapAlloc(v{} x {});",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        count.0,
                        inner_ty
                    )
                    .unwrap();
                }
                MemoryInstr::HeapFree { ptr } => {
                    writeln!(out, "free(v{})", ptr.0).unwrap();
                }
                MemoryInstr::Store { ptr, value } => {
                    writeln!(out, "*v{} = v{};", ptr.0, value.0).unwrap();
                }
                MemoryInstr::Load { dest, ptr } => {
                    writeln!(
                        out,
                        "v{}: {} = *v{};",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        ptr.0
                    )
                    .unwrap();
                }
                MemoryInstr::MemCopy { dest, src } => {
                    writeln!(
                        out,
                        "memcopy from address v{} to address v{};",
                        dest.0, src.0
                    )
                    .unwrap();
                }
                MemoryInstr::GetFieldPtr {
                    dest,
                    base_ptr,
                    field_index,
                } => {
                    let base_ty = cfg.values[base_ptr].ty;
                    let field_name = match interner.resolve(base_ty) {
                        Type::Pointer(to) => match interner.resolve(to) {
                            Type::Struct(s) => STRING_INTERNER
                                .resolve(s.fields(interner)[*field_index].0)
                                .to_string(),
                            _ => format!("{}", field_index),
                        },
                        _ => format!("{}", field_index),
                    };
                    writeln!(
                        out,
                        "v{}: {} = &v{}.{};",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        base_ptr.0,
                        field_name
                    )
                    .unwrap();
                }
                MemoryInstr::PtrOffset {
                    dest,
                    base_ptr,
                    index,
                } => {
                    writeln!(
                        out,
                        "v{}: {} = {} + v{};",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        base_ptr.0,
                        index.0
                    )
                    .unwrap();
                }
            },
            InstructionKind::Cast(kind) => match kind {
                CastInstr::SIToF { dest, src }
                | CastInstr::UIToF { dest, src }
                | CastInstr::FToSI { dest, src }
                | CastInstr::FToUI { dest, src }
                | CastInstr::FExt { dest, src }
                | CastInstr::FTrunc { dest, src }
                | CastInstr::Trunc { dest, src }
                | CastInstr::SExt { dest, src }
                | CastInstr::ZExt { dest, src }
                | CastInstr::BitCast { dest, src } => {
                    writeln!(
                        out,
                        "v{}: {} = {}(v{})",
                        dest.0,
                        get_vt(cfg, dest, interner),
                        get_cast_name(kind),
                        src.0
                    )
                    .unwrap();
                }
            },
            InstructionKind::Materialize(mat) => {
                writeln!(
                    out,
                    "v{}: {} = materialize {};",
                    mat.dest.0,
                    get_vt(cfg, &mat.dest, interner),
                    interner.to_string(interner.intern(&Type::Literal(mat.literal_type)))
                )
                .unwrap();
            }
        }
    }
}
