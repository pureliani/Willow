use std::collections::HashMap;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{
    BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType,
};
use inkwell::values::{BasicValueEnum, PhiValue};

use crate::globals::STRING_INTERNER;
use crate::hir::types::checked_declaration::{CheckedDeclaration, CheckedParam, FnType};
use crate::hir::utils::numeric::is_signed;
use crate::hir::{
    builders::{BasicBlockId, Function, Program, ValueId},
    instructions::{Instruction, Terminator},
    types::checked_type::Type,
};

pub mod emitters;

pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    program: &'ctx Program,

    // Maps HIR ValueId (SSA registers) to LLVM Values,
    // this is cleared at the start of every function.
    fn_values: HashMap<ValueId, BasicValueEnum<'ctx>>,

    // Maps HIR BasicBlockId to LLVM BasicBlocks,
    // this is cleared at the start of every function.
    fn_blocks: HashMap<BasicBlockId, inkwell::basic_block::BasicBlock<'ctx>>,

    current_fn: Option<&'ctx Function>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, program: &'ctx Program) -> Self {
        Self {
            context,
            module: context.create_module("main"),
            builder: context.create_builder(),
            program,
            fn_values: HashMap::new(),
            fn_blocks: HashMap::new(),
            current_fn: None,
        }
    }

    pub fn generate(&mut self) {
        for decl in self.program.declarations.values() {
            if let CheckedDeclaration::Function(f) = decl {
                self.gen_function_prototype(f);
            }
        }

        for decl in self.program.declarations.values() {
            if let CheckedDeclaration::Function(f) = decl {
                self.gen_function_body(f);
            }
        }

        self.module.print_to_stderr();
    }

    /// Converts a Lilac Type to an LLVM BasicType
    /// Returns None for Void
    fn lower_type(&self, ty: &Type) -> Option<BasicTypeEnum<'ctx>> {
        match ty {
            Type::Void => None,
            Type::Bool => Some(self.context.bool_type().into()),
            Type::I8 | Type::U8 => Some(self.context.i8_type().into()),
            Type::I16 | Type::U16 => Some(self.context.i16_type().into()),
            Type::I32 | Type::U32 => Some(self.context.i32_type().into()),
            Type::I64 | Type::U64 | Type::ISize | Type::USize => {
                Some(self.context.i64_type().into())
            }
            Type::F32 => Some(self.context.f32_type().into()),
            Type::F64 => Some(self.context.f64_type().into()),
            Type::Literal(lit) => self.lower_type(&lit.widen()),

            Type::String => unimplemented!("Codegen for String not yet implemented"),
            Type::List(_) => unimplemented!("Codegen for List not yet implemented"),
            Type::Struct(_) => Some(
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ),
            Type::Union(_) => {
                let tag_type = self.context.i16_type();
                let payload_type = self.context.i64_type();
                Some(
                    self.context
                        .struct_type(&[tag_type.into(), payload_type.into()], false)
                        .into(),
                )
            }
            Type::Null => unimplemented!("Codegen for Type::Null not yet implemented"),

            Type::Fn(_) => Some(
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ),

            Type::Unknown | Type::Never => {
                panic!("INTERNAL COMPILER ERROR: Invalid type in codegen")
            }
        }
    }

    /// Creates the function signature (name, return type, param types) in the LLVM module
    fn gen_function_prototype(&self, func: &Function) {
        let name = STRING_INTERNER.resolve(func.identifier.name);

        let fn_ty = FnType {
            params: func
                .params
                .iter()
                .map(|p| CheckedParam {
                    identifier: p.identifier.clone(),
                    ty: p.ty.clone(),
                })
                .collect(),
            return_type: Box::new(func.return_type.clone()),
        };

        let llvm_fn_type = self.lower_fn_type(&fn_ty);
        self.module.add_function(&name, llvm_fn_type, None);
    }

    fn gen_function_body(&mut self, func: &'ctx Function) {
        self.current_fn = Some(func);

        let name = STRING_INTERNER.resolve(func.identifier.name);
        let function = self.module.get_function(&name).unwrap();

        self.fn_values.clear();
        self.fn_blocks.clear();

        let mut phi_nodes: HashMap<ValueId, PhiValue<'ctx>> = HashMap::new();

        let entry_bb = self.context.append_basic_block(function, "entry");
        self.fn_blocks.insert(func.entry_block, entry_bb);

        for id in func.blocks.keys() {
            if *id == func.entry_block {
                continue;
            }
            let bb_name = format!("bb_{}", id.0);
            let bb = self.context.append_basic_block(function, &bb_name);
            self.fn_blocks.insert(*id, bb);
        }

        let mut llvm_param_index = 0;
        for param in &func.params {
            if self.lower_type(&param.ty).is_some() {
                let llvm_val = function.get_nth_param(llvm_param_index).unwrap();
                if let Some(val_id) = param.value_id {
                    // Set name for debugging IR
                    llvm_val.set_name(&STRING_INTERNER.resolve(param.identifier.name));
                    self.fn_values.insert(val_id, llvm_val);
                }
                llvm_param_index += 1;
            }
        }

        for (id, block) in &func.blocks {
            self.create_phis_for_block(block, &mut phi_nodes);

            let llvm_bb = self.fn_blocks.get(id).unwrap();
            self.builder.position_at_end(*llvm_bb);

            for instr in &block.instructions {
                self.gen_instruction(instr);
            }

            if let Some(term) = &block.terminator {
                self.gen_terminator(term);
            }
        }

        for block in func.blocks.values() {
            self.resolve_phis_for_block(block, &phi_nodes);
        }

        self.current_fn = None;
    }

    pub fn get_struct_layout(&self, fields: &[CheckedParam]) -> StructType<'ctx> {
        let field_types: Vec<BasicTypeEnum> = fields
            .iter()
            .filter_map(|field| self.lower_type(&field.ty))
            .collect();

        self.context.struct_type(&field_types, false)
    }

    pub fn lower_fn_type(&self, fn_ty: &FnType) -> FunctionType<'ctx> {
        let ret_type = self.lower_type(&fn_ty.return_type);

        let param_types: Vec<BasicMetadataTypeEnum> = fn_ty
            .params
            .iter()
            .filter_map(|p| self.lower_type(&p.ty))
            .map(|ty| ty.into())
            .collect();

        match ret_type {
            Some(rt) => rt.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        }
    }

    pub fn get_val_strict(&self, id: ValueId) -> BasicValueEnum<'ctx> {
        let llvm_val = *self.fn_values.get(&id).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: ValueId({:?}) not found in fn_values",
                id
            )
        });

        let hir_type = self.program.value_types.get(&id).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: ValueId({:?}) has no registered HIR type",
                id
            )
        });

        let expected_llvm_type = self.lower_type(hir_type).unwrap_or_else(|| {
            panic!(
                "INTERNAL COMPILER ERROR: HIR type {:?} lowers to Void/None, but was \
                 accessed as a value",
                hir_type
            )
        });

        if llvm_val.get_type() != expected_llvm_type {
            panic!(
                "INTERNAL COMPILER ERROR: Type Drift detected for ValueId({:?}).\nHIR \
                 Type: {:?}\nExpected LLVM Type: {:?}\nActual LLVM Type: {:?}",
                id,
                hir_type,
                expected_llvm_type,
                llvm_val.get_type()
            );
        }

        llvm_val
    }

    pub fn emit_cast(
        &self,
        val: BasicValueEnum<'ctx>,
        src_ty: &Type,
        dest_ty: &Type,
    ) -> BasicValueEnum<'ctx> {
        if src_ty == dest_ty {
            return val;
        }

        let llvm_dest_type = self
            .lower_type(dest_ty)
            .expect("Invalid destination type for cast");

        match (val, llvm_dest_type) {
            // Integer -> Integer (Widening/Narrowing)
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(int_dest_ty)) => {
                let src_width = int_val.get_type().get_bit_width();
                let dest_width = int_dest_ty.get_bit_width();

                if src_width == dest_width {
                    val
                } else if src_width > dest_width {
                    self.builder
                        .build_int_truncate(int_val, int_dest_ty, "trunc")
                        .unwrap()
                        .into()
                } else if is_signed(src_ty) {
                    self.builder
                        .build_int_s_extend(int_val, int_dest_ty, "sext")
                        .unwrap()
                        .into()
                } else {
                    self.builder
                        .build_int_z_extend(int_val, int_dest_ty, "zext")
                        .unwrap()
                        .into()
                }
            }

            // Float -> Float (Widening/Narrowing)
            (
                BasicValueEnum::FloatValue(float_val),
                BasicTypeEnum::FloatType(float_dest_ty),
            ) => self
                .builder
                .build_float_cast(float_val, float_dest_ty, "fpcast")
                .unwrap()
                .into(),

            (v, t) => panic!(
                "INTERNAL COMPILER ERROR: Invalid implicit cast requested.\nValue: \
                 {:?}\nSource HIR: {:?}\nTarget HIR: {:?}\nTarget LLVM: {:?}",
                v, src_ty, dest_ty, t
            ),
        }
    }

    fn gen_instruction(&mut self, instr: &Instruction) {
        match instr {
            Instruction::Const(c) => self.emit_const(c),
            Instruction::Binary(b) => self.emit_binary(b),
            Instruction::Unary(u) => self.emit_unary(u),
            Instruction::Comp(c) => self.emit_comp(c),
            Instruction::Cast(c) => self.emit_cast_instr(c),
            Instruction::Call(c) => self.emit_call(c),
            Instruction::Select(s) => self.emit_select(s),
            Instruction::Union(u) => self.emit_union(u),
            Instruction::Struct(_) => {
                unimplemented!("Struct instruction not implemented")
            }
            Instruction::List(_) => unimplemented!("List instruction not implemented"),
        }
    }

    fn gen_terminator(&mut self, term: &Terminator) {
        self.emit_terminator(term);
    }
}
