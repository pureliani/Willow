use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;
use inkwell::{FloatPredicate, IntPredicate};

use crate::codegen::CodeGenerator;
use crate::globals::STRING_INTERNER;
use crate::mir::builders::{FunctionCFG, ValueId};
use crate::mir::instructions::{
    BinaryInstr, CallInstr, CastInstr, CompInstr, Instruction, MaterializeInstr,
    MemoryInstr, SelectInstr, Terminator, UnaryInstr,
};
use crate::mir::types::checked_declaration::FnType;
use crate::mir::types::checked_type::{LiteralType, Type};
use crate::mir::utils::layout::get_layout_of;

impl<'ctx, 'a> CodeGenerator<'ctx, 'a> {
    fn get_val(&self, cfg: &FunctionCFG, id: ValueId) -> BasicValueEnum<'ctx> {
        if let Some(val) = self.values.get(&id) {
            return *val;
        }

        let ty_id = cfg.values[&id].ty;
        let ty = self.type_interner.resolve(ty_id);

        let layout = get_layout_of(
            &ty,
            self.type_interner,
            self.program.target_ptr_size,
            self.program.target_ptr_align,
        );

        if layout.size == 0 {
            return self.context.struct_type(&[], false).const_zero().into();
        }

        panic!(
            "INTERNAL COMPILER ERROR: ValueId({}) not found in codegen and is not a ZST ({})",
            id.0,
            self.type_interner.to_string(ty_id)
        );
    }

    fn store_val(&mut self, id: ValueId, val: BasicValueEnum<'ctx>) {
        self.values.insert(id, val);
    }

    pub fn emit_instruction(&mut self, cfg: &FunctionCFG, instr: &Instruction) {
        match instr {
            Instruction::Unary(unary) => self.emit_unary(cfg, unary),
            Instruction::Binary(binary) => self.emit_binary(cfg, binary),
            Instruction::Comp(comp) => self.emit_comp(cfg, comp),
            Instruction::Cast(cast) => self.emit_cast(cfg, cast),
            Instruction::Memory(memory) => self.emit_memory(cfg, memory),
            Instruction::Call(call) => self.emit_call(cfg, call),
            Instruction::Select(select) => self.emit_select(cfg, select),
            Instruction::Materialize(mat) => self.emit_materialize(cfg, mat),
        }
    }

    fn emit_materialize(&mut self, cfg: &FunctionCFG, mat: &MaterializeInstr) {
        let val: BasicValueEnum<'ctx> = match mat.literal_type {
            LiteralType::Bool(b) => {
                self.context.bool_type().const_int(b as u64, false).into()
            }
            LiteralType::U8(v) => {
                self.context.i8_type().const_int(v as u64, false).into()
            }
            LiteralType::U16(v) => {
                self.context.i16_type().const_int(v as u64, false).into()
            }
            LiteralType::U32(v) => {
                self.context.i32_type().const_int(v as u64, false).into()
            }
            LiteralType::U64(v) => self.context.i64_type().const_int(v, false).into(),
            LiteralType::USize(v) => {
                let target_data = self.target_machine.get_target_data();
                self.context
                    .ptr_sized_int_type(&target_data, None)
                    .const_int(v as u64, false)
                    .into()
            }
            LiteralType::I8(v) => self.context.i8_type().const_int(v as u64, true).into(),
            LiteralType::I16(v) => {
                self.context.i16_type().const_int(v as u64, true).into()
            }
            LiteralType::I32(v) => {
                self.context.i32_type().const_int(v as u64, true).into()
            }
            LiteralType::I64(v) => {
                self.context.i64_type().const_int(v as u64, true).into()
            }
            LiteralType::ISize(v) => {
                let target_data = self.target_machine.get_target_data();
                self.context
                    .ptr_sized_int_type(&target_data, None)
                    .const_int(v as u64, true)
                    .into()
            }
            LiteralType::F32(v) => self.context.f32_type().const_float(v.0 as f64).into(),
            LiteralType::F64(v) => self.context.f64_type().const_float(v.0).into(),
            LiteralType::Fn(decl_id) => {
                let func_val = self.functions[&decl_id];
                func_val.as_global_value().as_pointer_value().into()
            }
            LiteralType::String(str_id) => {
                let string_val = STRING_INTERNER.resolve(str_id);
                let len = string_val.len() as u64;

                let target_data = self.target_machine.get_target_data();
                let usize_ty = self.context.ptr_sized_int_type(&target_data, None);
                let len_val = usize_ty.const_int(len, false);

                let i8_ty = self.context.i8_type();
                let mut chars = Vec::with_capacity(string_val.len());
                for &b in string_val.as_bytes() {
                    chars.push(i8_ty.const_int(b as u64, false));
                }
                let array_val = i8_ty.const_array(&chars);

                let const_struct = self
                    .context
                    .const_struct(&[len_val.into(), array_val.into()], false);

                let global_str =
                    self.module
                        .add_global(const_struct.get_type(), None, "str_lit");
                global_str.set_initializer(&const_struct);
                global_str.set_constant(true);

                let dest_ty_id = cfg.values[&mat.dest].ty;
                let dest_ty = self.get_basic_type(dest_ty_id);

                self.builder
                    .build_pointer_cast(
                        global_str.as_pointer_value(),
                        dest_ty.into_pointer_type(),
                        "str_cast",
                    )
                    .unwrap()
                    .into()
            }

            LiteralType::Void
            | LiteralType::Never
            | LiteralType::Unknown
            | LiteralType::Null => panic!(
                "INTERNAL COMPILER ERROR: Cannot materialize literal type {:?}",
                mat.literal_type
            ),
        };

        self.store_val(mat.dest, val);
    }

    pub fn emit_terminator(&mut self, cfg: &FunctionCFG, terminator: &Terminator) {
        match terminator {
            Terminator::Jump { target } => {
                let llvm_block = self.blocks[target];
                self.builder.build_unconditional_branch(llvm_block).unwrap();
            }
            Terminator::CondJump {
                condition,
                true_target,
                false_target,
            } => {
                let cond_val = self.get_val(cfg, *condition).into_int_value();
                let true_block = self.blocks[true_target];
                let false_block = self.blocks[false_target];
                self.builder
                    .build_conditional_branch(cond_val, true_block, false_block)
                    .unwrap();
            }
            Terminator::Return { value } => {
                let ret_val = self.get_val(cfg, *value);
                self.builder.build_return(Some(&ret_val)).unwrap();
            }
            Terminator::Unreachable => {
                self.builder.build_unreachable().unwrap();
            }
        }
    }

    fn emit_unary(&mut self, cfg: &FunctionCFG, instr: &UnaryInstr) {
        match instr {
            UnaryInstr::INeg { dest, src } => {
                let val = self.get_val(cfg, *src).into_int_value();
                let res = self.builder.build_int_neg(val, "ineg").unwrap();
                self.store_val(*dest, res.into());
            }
            UnaryInstr::FNeg { dest, src } => {
                let val = self.get_val(cfg, *src).into_float_value();
                let res = self.builder.build_float_neg(val, "fneg").unwrap();
                self.store_val(*dest, res.into());
            }
            UnaryInstr::BNot { dest, src } => {
                let val = self.get_val(cfg, *src).into_int_value();
                let res = self.builder.build_not(val, "bnot").unwrap();
                self.store_val(*dest, res.into());
            }
        }
    }

    fn emit_binary(&mut self, cfg: &FunctionCFG, instr: &BinaryInstr) {
        macro_rules! int_binop {
            ($dest:expr, $lhs:expr, $rhs:expr, $builder_method:ident, $name:expr) => {{
                let l = self.get_val(cfg, *$lhs).into_int_value();
                let r = self.get_val(cfg, *$rhs).into_int_value();
                let res = self.builder.$builder_method(l, r, $name).unwrap();
                self.store_val(*$dest, res.into());
            }};
        }
        macro_rules! float_binop {
            ($dest:expr, $lhs:expr, $rhs:expr, $builder_method:ident, $name:expr) => {{
                let l = self.get_val(cfg, *$lhs).into_float_value();
                let r = self.get_val(cfg, *$rhs).into_float_value();
                let res = self.builder.$builder_method(l, r, $name).unwrap();
                self.store_val(*$dest, res.into());
            }};
        }

        match instr {
            BinaryInstr::IAdd { dest, lhs, rhs } => {
                int_binop!(dest, lhs, rhs, build_int_add, "iadd")
            }
            BinaryInstr::ISub { dest, lhs, rhs } => {
                int_binop!(dest, lhs, rhs, build_int_sub, "isub")
            }
            BinaryInstr::IMul { dest, lhs, rhs } => {
                int_binop!(dest, lhs, rhs, build_int_mul, "imul")
            }
            BinaryInstr::SDiv { dest, lhs, rhs } => {
                int_binop!(dest, lhs, rhs, build_int_signed_div, "sdiv")
            }
            BinaryInstr::UDiv { dest, lhs, rhs } => {
                int_binop!(dest, lhs, rhs, build_int_unsigned_div, "udiv")
            }
            BinaryInstr::SRem { dest, lhs, rhs } => {
                int_binop!(dest, lhs, rhs, build_int_signed_rem, "srem")
            }
            BinaryInstr::URem { dest, lhs, rhs } => {
                int_binop!(dest, lhs, rhs, build_int_unsigned_rem, "urem")
            }
            BinaryInstr::FAdd { dest, lhs, rhs } => {
                float_binop!(dest, lhs, rhs, build_float_add, "fadd")
            }
            BinaryInstr::FSub { dest, lhs, rhs } => {
                float_binop!(dest, lhs, rhs, build_float_sub, "fsub")
            }
            BinaryInstr::FMul { dest, lhs, rhs } => {
                float_binop!(dest, lhs, rhs, build_float_mul, "fmul")
            }
            BinaryInstr::FDiv { dest, lhs, rhs } => {
                float_binop!(dest, lhs, rhs, build_float_div, "fdiv")
            }
            BinaryInstr::FRem { dest, lhs, rhs } => {
                float_binop!(dest, lhs, rhs, build_float_rem, "frem")
            }
        }
    }

    fn emit_comp(&mut self, cfg: &FunctionCFG, instr: &CompInstr) {
        macro_rules! int_comp {
            ($dest:expr, $lhs:expr, $rhs:expr, $pred:expr, $name:expr) => {{
                let l = self.get_val(cfg, *$lhs).into_int_value();
                let r = self.get_val(cfg, *$rhs).into_int_value();
                let res = self.builder.build_int_compare($pred, l, r, $name).unwrap();
                self.store_val(*$dest, res.into());
            }};
        }
        macro_rules! float_comp {
            ($dest:expr, $lhs:expr, $rhs:expr, $pred:expr, $name:expr) => {{
                let l = self.get_val(cfg, *$lhs).into_float_value();
                let r = self.get_val(cfg, *$rhs).into_float_value();
                let res = self
                    .builder
                    .build_float_compare($pred, l, r, $name)
                    .unwrap();
                self.store_val(*$dest, res.into());
            }};
        }

        match instr {
            CompInstr::IEq { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::EQ, "ieq")
            }
            CompInstr::INeq { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::NE, "ineq")
            }
            CompInstr::SLt { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::SLT, "slt")
            }
            CompInstr::SLte { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::SLE, "slte")
            }
            CompInstr::SGt { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::SGT, "sgt")
            }
            CompInstr::SGte { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::SGE, "sgte")
            }
            CompInstr::ULt { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::ULT, "ult")
            }
            CompInstr::ULte { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::ULE, "ulte")
            }
            CompInstr::UGt { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::UGT, "ugt")
            }
            CompInstr::UGte { dest, lhs, rhs } => {
                int_comp!(dest, lhs, rhs, IntPredicate::UGE, "ugte")
            }
            CompInstr::FEq { dest, lhs, rhs } => {
                float_comp!(dest, lhs, rhs, FloatPredicate::OEQ, "feq")
            }
            CompInstr::FNeq { dest, lhs, rhs } => {
                float_comp!(dest, lhs, rhs, FloatPredicate::ONE, "fneq")
            }
            CompInstr::FLt { dest, lhs, rhs } => {
                float_comp!(dest, lhs, rhs, FloatPredicate::OLT, "flt")
            }
            CompInstr::FLte { dest, lhs, rhs } => {
                float_comp!(dest, lhs, rhs, FloatPredicate::OLE, "flte")
            }
            CompInstr::FGt { dest, lhs, rhs } => {
                float_comp!(dest, lhs, rhs, FloatPredicate::OGT, "fgt")
            }
            CompInstr::FGte { dest, lhs, rhs } => {
                float_comp!(dest, lhs, rhs, FloatPredicate::OGE, "fgte")
            }
        }
    }

    fn emit_cast(&mut self, cfg: &FunctionCFG, instr: &CastInstr) {
        match instr {
            CastInstr::SIToF { dest, src } => {
                let val = self.get_val(cfg, *src).into_int_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_float_type();
                let res = self
                    .builder
                    .build_signed_int_to_float(val, dest_ty, "sitof")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::UIToF { dest, src } => {
                let val = self.get_val(cfg, *src).into_int_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_float_type();
                let res = self
                    .builder
                    .build_unsigned_int_to_float(val, dest_ty, "uitof")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FToSI { dest, src } => {
                let val = self.get_val(cfg, *src).into_float_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_int_type();
                let res = self
                    .builder
                    .build_float_to_signed_int(val, dest_ty, "ftosi")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FToUI { dest, src } => {
                let val = self.get_val(cfg, *src).into_float_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_int_type();
                let res = self
                    .builder
                    .build_float_to_unsigned_int(val, dest_ty, "ftoui")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FExt { dest, src } => {
                let val = self.get_val(cfg, *src).into_float_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_float_type();
                let res = self.builder.build_float_ext(val, dest_ty, "fext").unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FTrunc { dest, src } => {
                let val = self.get_val(cfg, *src).into_float_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_float_type();
                let res = self
                    .builder
                    .build_float_trunc(val, dest_ty, "ftrunc")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::Trunc { dest, src } => {
                let val = self.get_val(cfg, *src).into_int_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_int_type();
                let res = self
                    .builder
                    .build_int_truncate(val, dest_ty, "trunc")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::SExt { dest, src } => {
                let val = self.get_val(cfg, *src).into_int_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_int_type();
                let res = self
                    .builder
                    .build_int_s_extend(val, dest_ty, "sext")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::ZExt { dest, src } => {
                let val = self.get_val(cfg, *src).into_int_value();
                let dest_ty = self.get_basic_type(cfg.values[dest].ty).into_int_type();
                let res = self
                    .builder
                    .build_int_z_extend(val, dest_ty, "zext")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::BitCast { dest, src } => {
                let val = self.get_val(cfg, *src);
                let dest_ty = self.get_basic_type(cfg.values[dest].ty);

                if val.get_type() == dest_ty {
                    self.store_val(*dest, val);
                } else {
                    let res = self
                        .builder
                        .build_bit_cast(val, dest_ty, "bitcast")
                        .unwrap();
                    self.store_val(*dest, res);
                }
            }
        }
    }

    fn emit_memory(&mut self, cfg: &FunctionCFG, instr: &MemoryInstr) {
        match instr {
            MemoryInstr::StackAlloc { dest, count } => {
                let dest_ty_id = cfg.values[dest].ty;
                let inner_ty_id = self.type_interner.unwrap_ptr(dest_ty_id);
                let inner_ty = self.get_basic_type(inner_ty_id);

                let count_val = self.context.i32_type().const_int(*count as u64, false);
                let ptr = self
                    .builder
                    .build_array_alloca(inner_ty, count_val, "stack_alloc")
                    .unwrap();
                self.store_val(*dest, ptr.into());
            }
            MemoryInstr::HeapAlloc { dest, count } => {
                let dest_ty_id = cfg.values[dest].ty;
                let inner_ty_id = self.type_interner.unwrap_ptr(dest_ty_id);
                let inner_ty = self.get_basic_type(inner_ty_id);

                let count_val = self.get_val(cfg, *count).into_int_value();

                let size_of = inner_ty.size_of().unwrap();
                let total_size = self
                    .builder
                    .build_int_mul(count_val, size_of, "alloc_size")
                    .unwrap();

                let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                    let ptr_sized_int = self
                        .context
                        .ptr_sized_int_type(&self.target_machine.get_target_data(), None);
                    let ptr_type =
                        self.context.ptr_type(inkwell::AddressSpace::default());
                    let fn_type = ptr_type.fn_type(&[ptr_sized_int.into()], false);
                    self.module.add_function("malloc", fn_type, None)
                });

                let raw_ptr = self
                    .builder
                    .build_call(malloc_fn, &[total_size.into()], "malloc_call")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_left();
                self.store_val(*dest, raw_ptr);
            }
            MemoryInstr::HeapFree { ptr } => {
                let ptr_val = self.get_val(cfg, *ptr).into_pointer_value();

                let free_fn = self.module.get_function("free").unwrap_or_else(|| {
                    let void_type = self.context.void_type();
                    let ptr_type =
                        self.context.ptr_type(inkwell::AddressSpace::default());
                    let fn_type = void_type.fn_type(&[ptr_type.into()], false);
                    self.module.add_function("free", fn_type, None)
                });

                self.builder
                    .build_call(free_fn, &[ptr_val.into()], "free_call")
                    .unwrap();
            }
            MemoryInstr::Store { ptr, value } => {
                let ptr_val = self.get_val(cfg, *ptr).into_pointer_value();
                let val = self.get_val(cfg, *value);
                self.builder.build_store(ptr_val, val).unwrap();
            }
            MemoryInstr::Load { dest, ptr } => {
                let ptr_val = self.get_val(cfg, *ptr).into_pointer_value();
                let dest_ty_id = cfg.values[dest].ty;
                let dest_ty = self.get_basic_type(dest_ty_id);

                let res = self.builder.build_load(dest_ty, ptr_val, "load").unwrap();
                self.store_val(*dest, res);
            }
            MemoryInstr::MemCopy { dest, src } => {
                let dest_ptr = self.get_val(cfg, *dest).into_pointer_value();
                let src_ptr = self.get_val(cfg, *src).into_pointer_value();

                let dest_ty_id = cfg.values[dest].ty;
                let inner_ty_id = self.type_interner.unwrap_ptr(dest_ty_id);
                let inner_ty = self.get_basic_type(inner_ty_id);

                let size = inner_ty.size_of().unwrap();
                let align = self
                    .target_machine
                    .get_target_data()
                    .get_abi_alignment(&inner_ty);

                self.builder
                    .build_memcpy(dest_ptr, align, src_ptr, align, size)
                    .unwrap();
            }
            MemoryInstr::GetFieldPtr {
                dest,
                base_ptr,
                field_index,
            } => {
                let base_ptr_val = self.get_val(cfg, *base_ptr).into_pointer_value();

                let base_ty_id = cfg.values[base_ptr].ty;
                let inner_ty_id = self.type_interner.unwrap_ptr(base_ty_id);
                let inner_ty = self.get_basic_type(inner_ty_id);

                let res = self
                    .builder
                    .build_struct_gep(
                        inner_ty,
                        base_ptr_val,
                        *field_index as u32,
                        "field_ptr",
                    )
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            MemoryInstr::PtrOffset {
                dest,
                base_ptr,
                index,
            } => {
                let base_ptr_val = self.get_val(cfg, *base_ptr).into_pointer_value();
                let index_val = self.get_val(cfg, *index).into_int_value();

                let base_ty_id = cfg.values[base_ptr].ty;
                let inner_ty_id = self.type_interner.unwrap_ptr(base_ty_id);
                let inner_ty = self.get_basic_type(inner_ty_id);

                let res = unsafe {
                    self.builder
                        .build_gep(inner_ty, base_ptr_val, &[index_val], "ptr_offset")
                        .unwrap()
                };
                self.store_val(*dest, res.into());
            }
        }
    }

    fn emit_call(&mut self, cfg: &FunctionCFG, instr: &CallInstr) {
        let func_ty_id = cfg.values[&instr.func].ty;
        let func_ty = self.type_interner.resolve(func_ty_id);

        let args: Vec<_> = instr
            .args
            .iter()
            .map(|arg| self.get_val(cfg, *arg).into())
            .collect();

        let call_site = match func_ty {
            Type::Literal(LiteralType::Fn(decl_id)) => {
                let func_val = self.functions[&decl_id];
                self.builder.build_call(func_val, &args, "call").unwrap()
            }
            Type::IndirectFn(FnType {
                params,
                return_type,
            }) => {
                let func_ptr = self.get_val(cfg, instr.func).into_pointer_value();

                let ret_ty = self.get_basic_type(return_type.id);
                let mut param_types = Vec::new();
                for p in params {
                    param_types.push(self.get_basic_type(p.ty.id).into());
                }

                let fn_type = ret_ty.fn_type(&param_types, false);

                self.builder
                    .build_indirect_call(fn_type, func_ptr, &args, "call")
                    .unwrap()
            }
            _ => panic!(
                "INTERNAL COMPILER ERROR: Call instruction target is not a function"
            ),
        };

        if let Some(res) = call_site.try_as_basic_value().left() {
            self.store_val(instr.dest, res);
        }
    }

    fn emit_select(&mut self, cfg: &FunctionCFG, instr: &SelectInstr) {
        let cond = self.get_val(cfg, instr.cond).into_int_value();
        let true_val = self.get_val(cfg, instr.true_val);
        let false_val = self.get_val(cfg, instr.false_val);

        let res = self
            .builder
            .build_select(cond, true_val, false_val, "select")
            .unwrap();
        self.store_val(instr.dest, res);
    }
}
