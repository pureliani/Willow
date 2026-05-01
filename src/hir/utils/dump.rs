use crate::{
    ast::decl::{Declaration, FnDecl},
    globals::STRING_INTERNER,
    hir::{
        builders::Program,
        instructions::{
            BasicBlock, BasicBlockId, BinaryOpKind, BuiltinFunction, FunctionCFG,
            InstrId, InstructionKind, MakeLiteralKind, MemoryInstr, Place, Terminator,
            UnaryOpKind,
        },
    },
};
use std::fmt::Write;

pub fn dump_program(program: &Program) {
    let mut out = String::new();
    writeln!(out, "========== HIR DUMP START ==========").unwrap();

    for (decl_id, cfg) in &program.cfgs {
        if let Some(Declaration::Fn(f)) = program.declarations.get(decl_id) {
            dump_function(f, cfg, &mut out);
        }
    }

    writeln!(out, "====================================").unwrap();
    println!("{}", out);
}

fn dump_function(f: &FnDecl, cfg: &FunctionCFG, out: &mut String) {
    let fn_name = STRING_INTERNER.resolve(f.identifier.name);
    writeln!(out, "fn {fn_name}:").unwrap();

    for (bid, bb) in &cfg.blocks {
        dump_block(bid, bb, cfg, out);
    }
}

pub fn dump_block(
    block_id: &BasicBlockId,
    bb: &BasicBlock,
    cfg: &FunctionCFG,
    out: &mut String,
) {
    writeln!(out, "  block_{}:", block_id.0).unwrap();

    if !bb.predecessors.is_empty() {
        write!(out, "    predecessors: ").unwrap();
        let preds: Vec<_> = bb
            .predecessors
            .iter()
            .map(|p| format!("block_{}", p.0))
            .collect();
        writeln!(out, "{}", preds.join(", ")).unwrap();
    }

    for (mem_id, sources) in &bb.memory_phis {
        let srcs: Vec<_> = sources
            .iter()
            .map(|s| format!("[m{}, block_{}]", s.memory.0, s.block.0))
            .collect();
        writeln!(out, "    m{} = memory_phi {}", mem_id.0, srcs.join(", ")).unwrap();
    }

    for &instr_id in &bb.instructions {
        dump_instruction(instr_id, cfg, out);
    }

    if let Some(term) = &bb.terminator {
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
                    "    cond_jmp v{} ? block_{} : block_{}",
                    condition.0, true_target.0, false_target.0
                )
                .unwrap();
            }
            Terminator::Return { value } => {
                writeln!(out, "    ret v{}", value.0).unwrap();
            }
            Terminator::Unreachable => {
                writeln!(out, "    unreachable").unwrap();
            }
        }
    }
    writeln!(out).unwrap();
}

fn format_place(place: &Place) -> String {
    match place {
        Place::Var(id) => format!("var_{}", id.0),
        Place::Expr(id) => format!("(v{})", id.0),
        Place::Field(base, name) => {
            format!(
                "{}.{}",
                format_place(base),
                STRING_INTERNER.resolve(name.name)
            )
        }
        Place::Index(base, instr_id) => {
            format!("{}[v{}]", format_place(base), instr_id.0)
        }
        Place::Deref(base) => format!("*{}", format_place(base)),
    }
}

pub fn dump_instruction(instr_id: InstrId, cfg: &FunctionCFG, out: &mut String) {
    let def = cfg.get_instr(instr_id);

    write!(out, "    v{} = ", instr_id.0).unwrap();

    match &def.kind {
        InstructionKind::CallBuiltin(c) => {
            let args: Vec<_> = c.args.iter().map(|a| format!("v{}", a.0)).collect();
            let name = match c.builtin {
                BuiltinFunction::StringConcat => "builtin_string_concat",
            };
            writeln!(
                out,
                "call {} ({}) [m{} -> m{}]",
                name,
                args.join(", "),
                c.memory_in.0,
                c.memory_out.0
            )
            .unwrap();
        }
        InstructionKind::MakeLiteral(lit) => match lit {
            MakeLiteralKind::Number(n) => writeln!(out, "{}", n.to_string()).unwrap(),
            MakeLiteralKind::Bool(b) => writeln!(out, "{}", b).unwrap(),
            MakeLiteralKind::String(s) => {
                writeln!(out, "\"{}\"", STRING_INTERNER.resolve(*s)).unwrap()
            }
            MakeLiteralKind::Fn(id) => writeln!(out, "fn_{}", id.0).unwrap(),
            MakeLiteralKind::Null => writeln!(out, "null").unwrap(),
            MakeLiteralKind::Void => writeln!(out, "void").unwrap(),
            MakeLiteralKind::Unknown => writeln!(out, "unknown").unwrap(),
            MakeLiteralKind::Never => writeln!(out, "never").unwrap(),
        },
        InstructionKind::Unary(u) => {
            let op_str = match u.op {
                UnaryOpKind::Neg => "-",
                UnaryOpKind::Not => "!",
            };
            writeln!(out, "{}v{}", op_str, u.value.0).unwrap();
        }
        InstructionKind::Binary(b) => {
            let op_str = match b.op {
                BinaryOpKind::Add => "+",
                BinaryOpKind::Sub => "-",
                BinaryOpKind::Mul => "*",
                BinaryOpKind::Div => "/",
                BinaryOpKind::Rem => "%",
                BinaryOpKind::Eq => "==",
                BinaryOpKind::Neq => "!=",
                BinaryOpKind::Lt => "<",
                BinaryOpKind::Lte => "<=",
                BinaryOpKind::Gt => ">",
                BinaryOpKind::Gte => ">=",
            };
            writeln!(out, "v{} {} v{}", b.lhs.0, op_str, b.rhs.0).unwrap();
        }
        InstructionKind::Cast(c) => {
            writeln!(out, "cast v{} to <type>", c.src.0).unwrap();
        }
        InstructionKind::Memory(m) => match m {
            MemoryInstr::StackAlloc { count, .. } => {
                writeln!(out, "stack_alloc {} items", count).unwrap()
            }
            MemoryInstr::HeapAlloc { count, .. } => {
                writeln!(out, "heap_alloc v{} items", count.0).unwrap()
            }
            MemoryInstr::HeapFree {
                ptr,
                memory_in,
                memory_out,
            } => writeln!(
                out,
                "free v{} [m{} -> m{}]",
                ptr.0, memory_in.0, memory_out.0
            )
            .unwrap(),
            MemoryInstr::MemCopy {
                from,
                to,
                memory_in,
                memory_out,
            } => writeln!(
                out,
                "memcopy v{} to v{} [m{} -> m{}]",
                from.0, to.0, memory_in.0, memory_out.0
            )
            .unwrap(),
            MemoryInstr::PtrOffset { base_ptr, index } => {
                writeln!(out, "v{} + offset v{}", base_ptr.0, index.0).unwrap()
            }
            MemoryInstr::ReadPlace { place, memory_in } => {
                writeln!(out, "read {} [m{}]", format_place(place), memory_in.0).unwrap()
            }
            MemoryInstr::WritePlace {
                place,
                value,
                memory_in,
                memory_out,
            } => writeln!(
                out,
                "write v{} to {} [m{} -> m{}]",
                value.0,
                format_place(place),
                memory_in.0,
                memory_out.0
            )
            .unwrap(),
        },
        InstructionKind::Call(c) => {
            let args: Vec<_> = c.args.iter().map(|a| format!("v{}", a.0)).collect();
            writeln!(
                out,
                "call v{}({}) [m{} -> m{}]",
                c.func.0,
                args.join(", "),
                c.memory_in.0,
                c.memory_out.0
            )
            .unwrap();
        }
        InstructionKind::Select(s) => {
            writeln!(
                out,
                "select v{} ? v{} : v{}",
                s.cond.0, s.true_val.0, s.false_val.0
            )
            .unwrap();
        }
        InstructionKind::Phi(p) => {
            let srcs: Vec<_> = p
                .sources
                .iter()
                .map(|s| format!("[v{}, block_{}]", s.value.0, s.block.0))
                .collect();
            writeln!(out, "phi {}", srcs.join(", ")).unwrap();
        }
        InstructionKind::IsType(i) => {
            writeln!(out, "is_type v{} <type>", i.src.0).unwrap();
        }
        InstructionKind::Param(idx) => {
            writeln!(out, "param {}", idx).unwrap();
        }
        InstructionKind::StructInit(s) => {
            let fields: Vec<_> = s
                .fields
                .iter()
                .map(|(n, v)| format!("{}: v{}", STRING_INTERNER.resolve(n.name), v.0))
                .collect();
            writeln!(out, "struct_init {{ {} }}", fields.join(", ")).unwrap();
        }
        InstructionKind::ListInit(l) => {
            let items: Vec<_> = l.items.iter().map(|v| format!("v{}", v.0)).collect();
            writeln!(out, "list_init [ {} ]", items.join(", ")).unwrap();
        }
        InstructionKind::GenericApply(g) => {
            writeln!(out, "generic_apply v{}<...>", g.func.0).unwrap();
        }
    }
}
