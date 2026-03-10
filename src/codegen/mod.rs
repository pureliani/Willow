use std::collections::HashMap;
use std::path::Path;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::targets::{FileType, TargetMachine};
use inkwell::types::{
    BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType,
};
use inkwell::values::{BasicValueEnum, PhiValue};

use crate::compile::interner::StringId;
use crate::globals::STRING_INTERNER;
use crate::hir::builders::FunctionBodyKind;
use crate::hir::types::checked_declaration::{CheckedDeclaration, CheckedParam, FnType};
use crate::hir::types::checked_type::LiteralType;
use crate::hir::types::ordered_number_kind::OrderedNumberKind;
use crate::hir::{
    builders::{BasicBlockId, Function, Program, ValueId},
    instructions::{Instruction, Terminator},
    types::checked_type::Type,
};
use crate::tokenize::NumberKind;

pub mod emitters;

pub struct StructLayout<'ctx> {
    pub llvm_type: StructType<'ctx>,
    /// Maps HIR field index -> LLVM field index, None if the field has no physical storage
    pub field_indices: Vec<Option<u32>>,
}

pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    program: &'ctx Program,
    target_machine: TargetMachine,

    // Maps HIR ValueId (SSA registers) to LLVM Values,
    // this is cleared at the start of every function.
    fn_values: HashMap<ValueId, BasicValueEnum<'ctx>>,

    // Maps HIR BasicBlockId to LLVM BasicBlocks,
    // this is cleared at the start of every function.
    fn_blocks: HashMap<BasicBlockId, inkwell::basic_block::BasicBlock<'ctx>>,

    current_fn: Option<&'ctx Function>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(
        context: &'ctx Context,
        program: &'ctx Program,
        target_machine: TargetMachine,
    ) -> Self {
        Self {
            context,
            module: context.create_module("main"),
            builder: context.create_builder(),
            program,
            fn_values: HashMap::new(),
            fn_blocks: HashMap::new(),
            current_fn: None,
            target_machine,
        }
    }

    pub fn generate(&mut self, path: &Path) {
        self.compile_all();
        self.write_object_file(path);
    }

    pub fn generate_ir(&mut self) {
        self.compile_all();
        self.module.print_to_stderr();
    }

    fn compile_all(&mut self) {
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
    }

    pub fn write_object_file(&self, path: &Path) {
        self.target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .unwrap();
    }

    /// Converts a Lilac Type to an LLVM BasicType
    /// Returns None for Void
    fn lower_type(&self, ty: &Type) -> Option<BasicTypeEnum<'ctx>> {
        match ty {
            Type::Void | Type::Null | Type::Literal(_) => None,
            Type::Bool => Some(self.context.bool_type().into()),
            Type::I8 | Type::U8 => Some(self.context.i8_type().into()),
            Type::I16 | Type::U16 => Some(self.context.i16_type().into()),
            Type::I32 | Type::U32 => Some(self.context.i32_type().into()),
            Type::I64 | Type::U64 | Type::ISize | Type::USize => {
                Some(self.context.i64_type().into())
            }
            Type::F32 => Some(self.context.f32_type().into()),
            Type::F64 => Some(self.context.f64_type().into()),
            Type::Struct(_) | Type::List(_) | Type::Fn(_) | Type::String => Some(
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ),
            Type::Union { base, .. } => {
                let (layout, _) = self.get_union_layout(base);
                Some(BasicTypeEnum::StructType(layout))
            }
            Type::Unknown | Type::Never => {
                panic!("INTERNAL COMPILER ERROR: Invalid type in codegen")
            }
        }
    }

    /// Creates the function signature (name, return type, param types) in the LLVM module
    fn gen_function_prototype(&self, func: &Function) {
        let name = STRING_INTERNER.resolve(func.identifier.name);

        if name == "main" {
            let i32_type = self.context.i32_type();
            let main_type = i32_type.fn_type(&[], false);
            self.module.add_function("main", main_type, None);
            return;
        }

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

        let linkage = match &func.body {
            FunctionBodyKind::External => Some(Linkage::External),
            FunctionBodyKind::Internal(_) => None,
            FunctionBodyKind::NotBuilt => {
                panic!("INTERNAL COMPILER ERROR: Codegen gen_function_prototype expected either an internal or external function")
            }
        };

        self.module.add_function(&name, llvm_fn_type, linkage);
    }

    fn gen_function_body(&mut self, func: &'ctx Function) {
        let FunctionBodyKind::Internal(cfg) = &func.body else {
            return;
        };

        self.current_fn = Some(func);

        let name = STRING_INTERNER.resolve(func.identifier.name);
        let function = self.module.get_function(&name).unwrap();

        self.fn_values.clear();
        self.fn_blocks.clear();

        let mut phi_nodes: HashMap<ValueId, PhiValue<'ctx>> = HashMap::new();

        let entry_bb = self.context.append_basic_block(function, "entry");
        self.fn_blocks.insert(cfg.entry_block, entry_bb);

        for id in cfg.blocks.keys() {
            if *id == cfg.entry_block {
                continue;
            }
            let bb_name = format!("bb_{}", id.0);
            let bb = self.context.append_basic_block(function, &bb_name);
            self.fn_blocks.insert(*id, bb);
        }

        let mut llvm_param_index = 0;
        for param in &func.params {
            if self.lower_type(&param.ty.kind).is_some() {
                let llvm_val = function.get_nth_param(llvm_param_index).unwrap();
                if let Some(val_id) = param.value_id {
                    // Set name for debugging IR
                    llvm_val.set_name(&STRING_INTERNER.resolve(param.identifier.name));
                    self.fn_values.insert(val_id, llvm_val);
                }
                llvm_param_index += 1;
            }
        }

        for (id, block) in &cfg.blocks {
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

        for block in cfg.blocks.values() {
            self.resolve_phis_for_block(block, &phi_nodes);
        }

        self.current_fn = None;
    }

    pub fn get_struct_layout(&self, fields: &[CheckedParam]) -> StructLayout<'ctx> {
        let mut llvm_fields = Vec::new();
        let mut field_indices = Vec::new();
        let mut llvm_index = 0u32;

        for field in fields {
            if self.lower_type(&field.ty.kind).is_some() {
                llvm_fields.push(self.lower_type(&field.ty.kind).unwrap());
                field_indices.push(Some(llvm_index));
                llvm_index += 1;
            } else {
                field_indices.push(None);
            }
        }

        StructLayout {
            llvm_type: self.context.struct_type(&llvm_fields, false),
            field_indices,
        }
    }

    pub fn lower_fn_type(&self, fn_ty: &FnType) -> FunctionType<'ctx> {
        let ret_type = self.lower_type(&fn_ty.return_type.kind);

        let param_types: Vec<BasicMetadataTypeEnum> = fn_ty
            .params
            .iter()
            .filter_map(|p| self.lower_type(&p.ty.kind))
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

    fn synth_number_const(&self, n: &OrderedNumberKind) -> BasicValueEnum<'ctx> {
        match n.0 {
            NumberKind::I8(v) => self.context.i8_type().const_int(v as u64, true).into(),
            NumberKind::I16(v) => {
                self.context.i16_type().const_int(v as u64, true).into()
            }
            NumberKind::I32(v) => {
                self.context.i32_type().const_int(v as u64, true).into()
            }
            NumberKind::I64(v) => {
                self.context.i64_type().const_int(v as u64, true).into()
            }
            NumberKind::ISize(v) => {
                self.context.i64_type().const_int(v as u64, true).into()
            }
            NumberKind::U8(v) => self.context.i8_type().const_int(v as u64, false).into(),
            NumberKind::U16(v) => {
                self.context.i16_type().const_int(v as u64, false).into()
            }
            NumberKind::U32(v) => {
                self.context.i32_type().const_int(v as u64, false).into()
            }
            NumberKind::U64(v) => self.context.i64_type().const_int(v, false).into(),
            NumberKind::USize(v) => {
                self.context.i64_type().const_int(v as u64, false).into()
            }
            NumberKind::F32(v) => self.context.f32_type().const_float(v as f64).into(),
            NumberKind::F64(v) => self.context.f64_type().const_float(v).into(),
        }
    }

    fn synth_string_const(&self, sid: StringId) -> BasicValueEnum<'ctx> {
        let s = STRING_INTERNER.resolve(sid);
        let bytes = s.as_bytes(); // UTF-8 by virtue of Rust's str
        let len = bytes.len() as u64;

        let desc_name = format!("str.desc.{}", sid.0);
        if let Some(existing) = self.module.get_global(&desc_name) {
            return existing.as_pointer_value().into();
        }

        let i8_type = self.context.i8_type();
        let buf_name = format!("str.buf.{}", sid.0);
        let buf_global = {
            let array_type = i8_type.array_type(bytes.len() as u32);
            let global = self.module.add_global(array_type, None, &buf_name);
            global.set_constant(true);
            global.set_linkage(inkwell::module::Linkage::Private);
            let chars: Vec<_> = bytes
                .iter()
                .map(|&b| i8_type.const_int(b as u64, false))
                .collect();
            global.set_initializer(&i8_type.const_array(&chars));
            global.as_pointer_value()
        };

        // { len: usize, cap: usize, ptr: ptr<u8> }
        let usize_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let desc_type = self.context.struct_type(
            &[usize_type.into(), usize_type.into(), ptr_type.into()],
            false,
        );

        let desc_global = self.module.add_global(desc_type, None, &desc_name);
        desc_global.set_constant(true);
        desc_global.set_linkage(inkwell::module::Linkage::Private);
        desc_global.set_initializer(&desc_type.const_named_struct(&[
            usize_type.const_int(len, false).into(), // len = byte length in UTF-8
            usize_type.const_int(0, false).into(),   // cap = 0, signals non-owning
            buf_global.into(),                       // ptr to UTF-8 buffer
        ]));

        desc_global.as_pointer_value().into()
    }

    pub fn get_val(&self, id: ValueId) -> Option<BasicValueEnum<'ctx>> {
        match self.program.value_types.get(&id)? {
            Type::Literal(lit) => match lit {
                LiteralType::Number(n) => Some(self.synth_number_const(n)),
                LiteralType::Bool(b) => {
                    Some(self.context.bool_type().const_int(*b as u64, false).into())
                }
                LiteralType::String(sid) => Some(self.synth_string_const(*sid)),
            },
            Type::Null | Type::Void => None,

            _ => Some(self.get_val_strict(id)),
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
            Instruction::Struct(s) => {
                self.emit_struct(s);
            }
            Instruction::List(l) => self.emit_list(l),
            Instruction::BitCast(bc) => {
                if let Some(val) = self.get_val(bc.src) {
                    self.fn_values.insert(bc.dest, val);
                }
            }
        }
    }

    fn gen_terminator(&mut self, term: &Terminator) {
        self.emit_terminator(term);
    }
}
