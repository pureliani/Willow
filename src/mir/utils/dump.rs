use crate::{
    globals::STRING_INTERNER,
    mir::{
        builders::{
            BasicBlockId, ExpectBody, Function, FunctionBodyKind, FunctionCFG, Program,
            ValueId,
        },
        instructions::{
            BinaryInstr, CallInstr, CompInstr, Instruction, MemoryInstr, SelectInstr,
            Terminator, UnaryInstr,
        },
        types::{checked_declaration::CheckedDeclaration, checked_type::Type},
        utils::type_to_string::type_to_string,
    },
};
use std::{collections::VecDeque, fmt::Write};

fn get_vt(p: &Program, vid: &ValueId) -> String {
    type_to_string(&p.value_types[vid])
}

fn find_blocks(f: &Function) -> Vec<BasicBlockId> {
    let mut blocks = Vec::new();
    let mut queue = VecDeque::new();
    let mut expanded = std::collections::HashSet::new();

    if let FunctionBodyKind::Internal(FunctionCFG {
        entry_block: f_entry_block,
        blocks: f_blocks,
        ..
    }) = &f.body
    {
        queue.push_back(*f_entry_block);

        while let Some(bid) = queue.pop_front() {
            blocks.retain(|&id| id != bid);
            blocks.push(bid);

            if expanded.insert(bid) {
                if let Some(bb) = f_blocks.get(&bid) {
                    if let Some(terminator) = &bb.terminator {
                        match terminator {
                            Terminator::Jump { target, .. } => {
                                queue.push_back(*target);
                            }
                            Terminator::CondJump {
                                true_target,
                                false_target,
                                ..
                            } => {
                                queue.push_back(*true_target);
                                queue.push_back(*false_target);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    blocks
}

pub fn dump_program(program: &Program) {
    let mut out = String::new();
    writeln!(out, "========== HIR DUMP START ==========").unwrap();
    for (_, decl) in program.declarations.iter() {
        if let CheckedDeclaration::Function(f) = decl {
            dump_function(f, program, &mut out);
        }
    }
    writeln!(out, "====================================").unwrap();
    println!("{}", out);
}

fn dump_function(f: &Function, p: &Program, out: &mut String) {
    let fn_name = STRING_INTERNER.resolve(f.identifier.name);
    let return_type = type_to_string(&f.return_type.kind);
    writeln!(out, "fn {fn_name} -> {return_type}:").unwrap();
    let block_ids = find_blocks(f);

    for bid in block_ids {
        dump_block(&bid, f, p, out);
    }
}

pub fn dump_block(block_id: &BasicBlockId, f: &Function, p: &Program, out: &mut String) {
    let bb = f.expect_body().blocks.get(block_id).unwrap();
    writeln!(out, "  block_{}:", bb.id.0).unwrap();

    writeln!(out, "    predecessors {{ ").unwrap();
    for p in &bb.predecessors {
        writeln!(out, "      block_{}", p.0).unwrap();
    }
    writeln!(out, "    }} ").unwrap();

    writeln!(out).unwrap();

    dump_instructions(&bb.instructions, p, out);

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
        }
    }
}

pub fn dump_instructions(instrs: &[Instruction], p: &Program, out: &mut String) {
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

    for instruction in instrs {
        write!(out, "    ").unwrap();
        match instruction {
            Instruction::Unary(kind) => match kind {
                UnaryInstr::INeg { dest, src } | UnaryInstr::FNeg { dest, src } => {
                    writeln!(out, "v{}: {} = -{};", dest.0, get_vt(p, dest), src.0)
                        .unwrap();
                }
                UnaryInstr::BNot { dest, src } => {
                    writeln!(out, "v{}: {} = !{};", dest.0, get_vt(p, dest), src.0)
                        .unwrap();
                }
            },
            Instruction::Binary(kind) => match kind {
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
                        get_vt(p, dest),
                        lhs.0,
                        get_binary_sign(kind),
                        rhs.0
                    )
                    .unwrap();
                }
            },
            Instruction::Comp(kind) => match kind {
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
                        get_vt(p, dest),
                        lhs.0,
                        get_comp_sign(kind),
                        rhs.0
                    )
                    .unwrap();
                }
            },
            Instruction::Select(SelectInstr {
                dest,
                cond,
                true_val,
                false_val,
            }) => {
                writeln!(
                    out,
                    "v{}: {} = v{} ? v{} : v{};",
                    dest.0,
                    get_vt(p, dest),
                    cond.0,
                    true_val.0,
                    false_val.0
                )
                .unwrap();
            }
            Instruction::Call(CallInstr { dest, func, args }) => {
                let args = args
                    .iter()
                    .map(|a| format!("v{}", a.0))
                    .collect::<Vec<String>>()
                    .join(", ");

                writeln!(
                    out,
                    "v{}: {} = call v{}({});",
                    dest.0,
                    get_vt(p, dest),
                    func.0,
                    args
                )
                .unwrap();
            }
            Instruction::Reinterpret(bitcast_instr) => {
                let dest_type_str = get_vt(p, &bitcast_instr.dest);
                writeln!(
                    out,
                    "v{}: {} = bitcast v{};",
                    bitcast_instr.dest.0, dest_type_str, bitcast_instr.src.0,
                )
                .unwrap();
            }
            Instruction::Memory(kind) => match kind {
                MemoryInstr::StackAlloc { dest, count } => {
                    let inner_ty = match &p.value_types[dest] {
                        Type::Pointer(to) => type_to_string(to),
                        _ => "unknown".to_string(),
                    };
                    writeln!(
                        out,
                        "v{}: {} = stackAlloc(v{} x {});",
                        dest.0,
                        get_vt(p, dest),
                        count,
                        inner_ty
                    )
                    .unwrap();
                }
                MemoryInstr::HeapAlloc { dest, count } => {
                    let inner_ty = match &p.value_types[dest] {
                        Type::Pointer(to) => type_to_string(to),
                        _ => "unknown".to_string(),
                    };
                    writeln!(
                        out,
                        "v{}: {} = heapAlloc(v{} x {});",
                        dest.0,
                        get_vt(p, dest),
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
                    writeln!(out, "v{}: {} = *v{};", dest.0, get_vt(p, dest), ptr.0)
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
                    let base_ty = &p.value_types[base_ptr];
                    let field_name = match base_ty {
                        Type::Pointer(to) => match &**to {
                            Type::Struct(s) => {
                                STRING_INTERNER.resolve(s.fields()[*field_index].0)
                            }
                            _ => format!("{}", field_index),
                        },
                        _ => format!("{}", field_index),
                    };
                    writeln!(
                        out,
                        "v{}: {} = &v{}.{};",
                        dest.0,
                        get_vt(p, dest),
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
                        "v{}: {} = {} + {};",
                        dest.0,
                        get_vt(p, dest),
                        base_ptr.0,
                        index.0
                    )
                    .unwrap();
                }
            },
        }
    }
}
