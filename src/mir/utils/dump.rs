use crate::{
    globals::STRING_INTERNER,
    mir::{
        builders::{
            BasicBlockId, ExpectBody, Function, FunctionBodyKind, FunctionCFG, Program,
            ValueId,
        },
        instructions::{
            BinaryInstr, CallInstr, CompInstr, ConstInstr, Instruction, ListInstr,
            SelectInstr, StructInstr, Terminator, UnaryInstr, UnionInstr,
        },
        types::checked_declaration::CheckedDeclaration,
        utils::type_to_string::type_to_string,
    },
    tokenize::number_kind_to_suffix,
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

    writeln!(out, "    phis {{ ").unwrap();
    for (dest, operands) in &bb.phis {
        let ops_str = operands
            .iter()
            .map(|phi| format!("v{} from block_{}", phi.value.0, phi.from.0))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "      v{}: {} = phi [ {} ];",
            dest.0,
            get_vt(p, dest),
            ops_str
        )
        .unwrap();
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
        BinaryInstr::Add { .. } => "+",
        BinaryInstr::Sub { .. } => "-",
        BinaryInstr::Mul { .. } => "*",
        BinaryInstr::Div { .. } => "/",
        BinaryInstr::Rem { .. } => "%",
    };

    let get_comp_sign = |instr: &CompInstr| match instr {
        CompInstr::Eq { .. } => "==",
        CompInstr::Neq { .. } => "!=",
        CompInstr::Lt { .. } => "<",
        CompInstr::Lte { .. } => "<=",
        CompInstr::Gt { .. } => ">",
        CompInstr::Gte { .. } => ">=",
    };

    for instruction in instrs {
        write!(out, "    ").unwrap();
        match instruction {
            Instruction::Const(kind) => match kind {
                ConstInstr::ConstNumber { dest, val } => {
                    writeln!(
                        out,
                        "v{}: {} = {};",
                        dest.0,
                        number_kind_to_suffix(val),
                        val.to_string()
                    )
                    .unwrap();
                }
                ConstInstr::ConstBool { dest, val } => {
                    writeln!(out, "v{}: bool = {};", dest.0, val).unwrap();
                }
                ConstInstr::ConstString { dest, val } => {
                    let literal = STRING_INTERNER.resolve(*val);
                    writeln!(out, "v{}: string = \"{}\";", dest.0, literal).unwrap();
                }
                ConstInstr::ConstFn { dest, decl_id } => {
                    let decl = p.declarations.get(decl_id).unwrap_or_else(|| {
                        panic!(
                            "INTERNAL COMPILER ERROR: No corresponding for \
                             DeclarationId({})",
                            decl_id.0
                        )
                    });
                    let fn_identifier = if let CheckedDeclaration::Function(f) = decl {
                        f.identifier.clone()
                    } else {
                        panic!(
                            "INTERNAL COMPILER ERROR: Expected declaration id to \
                             correspond to a function"
                        )
                    };

                    writeln!(
                        out,
                        "v{}: {} = <function {} from {}>;",
                        dest.0,
                        get_vt(p, dest),
                        STRING_INTERNER.resolve(fn_identifier.name),
                        fn_identifier.span.path.0.display()
                    )
                    .unwrap();
                }
            },
            Instruction::Unary(kind) => match kind {
                UnaryInstr::Neg { dest, src } => {
                    writeln!(out, "v{}: {} = -{};", dest.0, get_vt(p, dest), src.0)
                        .unwrap();
                }
                UnaryInstr::Not { dest, src } => {
                    writeln!(out, "v{}: {} = !{};", dest.0, get_vt(p, dest), src.0)
                        .unwrap();
                }
            },
            Instruction::Binary(kind) => match kind {
                BinaryInstr::Add { dest, lhs, rhs }
                | BinaryInstr::Sub { dest, lhs, rhs }
                | BinaryInstr::Mul { dest, lhs, rhs }
                | BinaryInstr::Div { dest, lhs, rhs }
                | BinaryInstr::Rem { dest, lhs, rhs } => {
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
                CompInstr::Eq { dest, lhs, rhs }
                | CompInstr::Neq { dest, lhs, rhs }
                | CompInstr::Lt { dest, lhs, rhs }
                | CompInstr::Lte { dest, lhs, rhs }
                | CompInstr::Gt { dest, lhs, rhs }
                | CompInstr::Gte { dest, lhs, rhs } => {
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
            Instruction::Struct(struct_instr) => match struct_instr {
                StructInstr::Construct { dest, fields } => {
                    let fields_str = fields
                        .iter()
                        .map(|(name, val)| {
                            format!("{}: v{}", STRING_INTERNER.resolve(*name), val.0)
                        })
                        .collect::<Vec<String>>()
                        .join(", ");
                    writeln!(
                        out,
                        "v{}: {} = struct {{ {} }};",
                        dest.0,
                        get_vt(p, dest),
                        fields_str
                    )
                    .unwrap();
                }
                StructInstr::ReadField { dest, base, field } => {
                    writeln!(
                        out,
                        "v{}: {} = v{}.{};",
                        dest.0,
                        get_vt(p, dest),
                        base.0,
                        STRING_INTERNER.resolve(*field)
                    )
                    .unwrap();
                }
                StructInstr::UpdateField {
                    dest,
                    base,
                    field,
                    value,
                } => {
                    writeln!(
                        out,
                        "v{}: {} = update v{} {{ {}: v{} }};",
                        dest.0,
                        get_vt(p, dest),
                        base.0,
                        STRING_INTERNER.resolve(*field),
                        value.0
                    )
                    .unwrap();
                }
            },
            Instruction::Union(union_instr) => match union_instr {
                UnionInstr::TestVariant {
                    dest,
                    src,
                    variant_type,
                } => {
                    writeln!(
                        out,
                        "v{}: {} = test_variant v{} is {};",
                        dest.0,
                        get_vt(p, dest),
                        src.0,
                        type_to_string(variant_type)
                    )
                    .unwrap();
                }
            },
            Instruction::List(list_instr) => match list_instr {
                ListInstr::Init {
                    dest,
                    element_type: _,
                    items,
                } => {
                    let items_str = items
                        .iter()
                        .map(|v| format!("v{}", v.0))
                        .collect::<Vec<String>>()
                        .join(", ");
                    writeln!(out, "v{}: {} = [{}];", dest.0, get_vt(p, dest), items_str)
                        .unwrap();
                }
                ListInstr::Get { dest, list, index } => {
                    writeln!(
                        out,
                        "v{}: {} = v{}::get({});",
                        dest.0,
                        get_vt(p, dest),
                        list.0,
                        index.0
                    )
                    .unwrap();
                }
                ListInstr::GetUnsafe { dest, list, index } => {
                    writeln!(
                        out,
                        "v{}: {} = v{}::getUnsafe({});",
                        dest.0,
                        get_vt(p, dest),
                        list.0,
                        index.0
                    )
                    .unwrap();
                }
                ListInstr::Set {
                    dest,
                    list,
                    index,
                    value,
                } => {
                    writeln!(
                        out,
                        "v{}: {} = setListItem(v{}[{}] to v{});",
                        dest.0,
                        get_vt(p, dest),
                        list.0,
                        index.0,
                        value.0
                    )
                    .unwrap();
                }
                ListInstr::Len { dest, list } => {
                    writeln!(out, "v{}: {} = len(v{});", dest.0, get_vt(p, dest), list.0)
                        .unwrap();
                }
            },
            Instruction::Cast(cast_instr) => {
                let dest_type_str = get_vt(p, &cast_instr.dest);
                writeln!(
                    out,
                    "v{}: {} = v{}::as({})",
                    cast_instr.dest.0, dest_type_str, cast_instr.src.0, dest_type_str,
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
        }
    }
}
