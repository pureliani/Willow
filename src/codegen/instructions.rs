use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;
use inkwell::{FloatPredicate, IntPredicate};

use crate::codegen::CodeGenerator;
use crate::mir::builders::ValueId;
use crate::mir::instructions::{
    BinaryInstr, CallInstr, CastInstr, CompInstr, Instruction, MemoryInstr, SelectInstr,
    Terminator, UnaryInstr,
};
use crate::mir::types::checked_declaration::FnType;
use crate::mir::types::checked_type::Type;

impl<'ctx, 'a> CodeGenerator<'ctx, 'a> {
    fn get_val(&self, id: ValueId) -> BasicValueEnum<'ctx> {
        *self.values.get(&id).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: ValueId({}) not found in codegen",
                id.0
            )
        })
    }

    fn store_val(&mut self, id: ValueId, val: BasicValueEnum<'ctx>) {
        self.values.insert(id, val);
    }

    pub fn emit_instruction(&mut self, instr: &Instruction) {
        match instr {
            Instruction::Unary(unary) => self.emit_unary(unary),
            Instruction::Binary(binary) => self.emit_binary(binary),
            Instruction::Comp(comp) => self.emit_comp(comp),
            Instruction::Cast(cast) => self.emit_cast(cast),
            Instruction::Memory(memory) => self.emit_memory(memory),
            Instruction::Call(call) => self.emit_call(call),
            Instruction::Select(select) => self.emit_select(select),
        }
    }

    pub fn emit_terminator(&mut self, terminator: &Terminator) {
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
                let cond_val = self.get_val(*condition).into_int_value();
                let true_block = self.blocks[true_target];
                let false_block = self.blocks[false_target];
                self.builder
                    .build_conditional_branch(cond_val, true_block, false_block)
                    .unwrap();
            }
            Terminator::Return { value } => {
                let val_ty = self.program.value_types[value];
                let resolved_ty = self.type_interner.resolve(val_ty);

                if matches!(resolved_ty, Type::Void | Type::Never | Type::Null) {
                    self.builder.build_return(None).unwrap();
                } else {
                    let ret_val = self.get_val(*value);
                    self.builder.build_return(Some(&ret_val)).unwrap();
                }
            }
            Terminator::Panic { message: _ } => {
                let trap_fn =
                    self.module.get_function("llvm.trap").unwrap_or_else(|| {
                        let void_type = self.context.void_type();
                        let fn_type = void_type.fn_type(&[], false);
                        self.module.add_function("llvm.trap", fn_type, None)
                    });
                self.builder.build_call(trap_fn, &[], "panic_trap").unwrap();
                self.builder.build_unreachable().unwrap();
            }
        }
    }

    fn emit_unary(&mut self, instr: &UnaryInstr) {
        match instr {
            UnaryInstr::INeg { dest, src } => {
                let val = self.get_val(*src).into_int_value();
                let res = self.builder.build_int_neg(val, "ineg").unwrap();
                self.store_val(*dest, res.into());
            }
            UnaryInstr::FNeg { dest, src } => {
                let val = self.get_val(*src).into_float_value();
                let res = self.builder.build_float_neg(val, "fneg").unwrap();
                self.store_val(*dest, res.into());
            }
            UnaryInstr::BNot { dest, src } => {
                let val = self.get_val(*src).into_int_value();
                let res = self.builder.build_not(val, "bnot").unwrap();
                self.store_val(*dest, res.into());
            }
        }
    }

    fn emit_binary(&mut self, instr: &BinaryInstr) {
        macro_rules! int_binop {
            ($dest:expr, $lhs:expr, $rhs:expr, $builder_method:ident, $name:expr) => {{
                let l = self.get_val(*$lhs).into_int_value();
                let r = self.get_val(*$rhs).into_int_value();
                let res = self.builder.$builder_method(l, r, $name).unwrap();
                self.store_val(*$dest, res.into());
            }};
        }
        macro_rules! float_binop {
            ($dest:expr, $lhs:expr, $rhs:expr, $builder_method:ident, $name:expr) => {{
                let l = self.get_val(*$lhs).into_float_value();
                let r = self.get_val(*$rhs).into_float_value();
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

    fn emit_comp(&mut self, instr: &CompInstr) {
        macro_rules! int_comp {
            ($dest:expr, $lhs:expr, $rhs:expr, $pred:expr, $name:expr) => {{
                let l = self.get_val(*$lhs).into_int_value();
                let r = self.get_val(*$rhs).into_int_value();
                let res = self.builder.build_int_compare($pred, l, r, $name).unwrap();
                self.store_val(*$dest, res.into());
            }};
        }
        macro_rules! float_comp {
            ($dest:expr, $lhs:expr, $rhs:expr, $pred:expr, $name:expr) => {{
                let l = self.get_val(*$lhs).into_float_value();
                let r = self.get_val(*$rhs).into_float_value();
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

    fn emit_cast(&mut self, instr: &CastInstr) {
        match instr {
            CastInstr::SIToF { dest, src } => {
                let val = self.get_val(*src).into_int_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_float_type();
                let res = self
                    .builder
                    .build_signed_int_to_float(val, dest_ty, "sitof")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::UIToF { dest, src } => {
                let val = self.get_val(*src).into_int_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_float_type();
                let res = self
                    .builder
                    .build_unsigned_int_to_float(val, dest_ty, "uitof")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FToSI { dest, src } => {
                let val = self.get_val(*src).into_float_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_int_type();
                let res = self
                    .builder
                    .build_float_to_signed_int(val, dest_ty, "ftosi")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FToUI { dest, src } => {
                let val = self.get_val(*src).into_float_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_int_type();
                let res = self
                    .builder
                    .build_float_to_unsigned_int(val, dest_ty, "ftoui")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FExt { dest, src } => {
                let val = self.get_val(*src).into_float_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_float_type();
                let res = self.builder.build_float_ext(val, dest_ty, "fext").unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::FTrunc { dest, src } => {
                let val = self.get_val(*src).into_float_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_float_type();
                let res = self
                    .builder
                    .build_float_trunc(val, dest_ty, "ftrunc")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::Trunc { dest, src } => {
                let val = self.get_val(*src).into_int_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_int_type();
                let res = self
                    .builder
                    .build_int_truncate(val, dest_ty, "trunc")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::SExt { dest, src } => {
                let val = self.get_val(*src).into_int_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_int_type();
                let res = self
                    .builder
                    .build_int_s_extend(val, dest_ty, "sext")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::ZExt { dest, src } => {
                let val = self.get_val(*src).into_int_value();
                let dest_ty = self
                    .get_basic_type(self.program.value_types[dest])
                    .into_int_type();
                let res = self
                    .builder
                    .build_int_z_extend(val, dest_ty, "zext")
                    .unwrap();
                self.store_val(*dest, res.into());
            }
            CastInstr::BitCast { dest, src } => {
                let val = self.get_val(*src);
                let dest_ty = self.get_basic_type(self.program.value_types[dest]);
                let res = self
                    .builder
                    .build_bit_cast(val, dest_ty, "bitcast")
                    .unwrap();
                self.store_val(*dest, res);
            }
        }
    }

    fn emit_memory(&mut self, instr: &MemoryInstr) {
        match instr {
            MemoryInstr::StackAlloc { dest, count } => {
                let dest_ty_id = self.program.value_types[dest];
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
                let dest_ty_id = self.program.value_types[dest];
                let inner_ty_id = self.type_interner.unwrap_ptr(dest_ty_id);
                let inner_ty = self.get_basic_type(inner_ty_id);

                let count_val = self.get_val(*count).into_int_value();

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
                let ptr_val = self.get_val(*ptr).into_pointer_value();

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
                let ptr_val = self.get_val(*ptr).into_pointer_value();
                let val = self.get_val(*value);
                self.builder.build_store(ptr_val, val).unwrap();
            }
            MemoryInstr::Load { dest, ptr } => {
                let ptr_val = self.get_val(*ptr).into_pointer_value();
                let dest_ty_id = self.program.value_types[dest];
                let dest_ty = self.get_basic_type(dest_ty_id);

                let res = self.builder.build_load(dest_ty, ptr_val, "load").unwrap();
                self.store_val(*dest, res);
            }
            MemoryInstr::MemCopy { dest, src } => {
                let dest_ptr = self.get_val(*dest).into_pointer_value();
                let src_ptr = self.get_val(*src).into_pointer_value();

                let dest_ty_id = self.program.value_types[dest];
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
                let base_ptr_val = self.get_val(*base_ptr).into_pointer_value();

                let base_ty_id = self.program.value_types[base_ptr];
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
                let base_ptr_val = self.get_val(*base_ptr).into_pointer_value();
                let index_val = self.get_val(*index).into_int_value();

                let base_ty_id = self.program.value_types[base_ptr];
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

    fn emit_call(&mut self, instr: &CallInstr) {
        let func_ty_id = self.program.value_types[&instr.func];
        let func_ty = self.type_interner.resolve(func_ty_id);

        let args: Vec<_> = instr
            .args
            .iter()
            .map(|arg| self.get_val(*arg).into())
            .collect();

        let call_site = match func_ty {
            Type::Fn(FnType::Direct(decl_id)) => {
                let func_val = self.functions[&decl_id];
                self.builder.build_call(func_val, &args, "call").unwrap()
            }
            Type::Fn(FnType::Indirect {
                params,
                return_type,
            }) => {
                let func_ptr = self.get_val(instr.func).into_pointer_value();

                // Construct the FunctionType for the indirect call
                let ret_ty = self.get_any_type(return_type.id);
                let mut param_types = Vec::new();
                for p in params {
                    param_types.push(self.get_basic_type(p.ty.id).into());
                }

                let fn_type = if ret_ty.is_void_type() {
                    self.context.void_type().fn_type(&param_types, false)
                } else {
                    let basic_ret = self.get_basic_type(return_type.id);
                    basic_ret.fn_type(&param_types, false)
                };

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

    fn emit_select(&mut self, instr: &SelectInstr) {
        let cond = self.get_val(instr.cond).into_int_value();
        let true_val = self.get_val(instr.true_val);
        let false_val = self.get_val(instr.false_val);

        let res = self
            .builder
            .build_select(cond, true_val, false_val, "select")
            .unwrap();
        self.store_val(instr.dest, res);
    }
}
