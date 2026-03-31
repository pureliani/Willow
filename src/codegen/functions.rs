use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};

use crate::codegen::CodeGenerator;
use crate::globals::STRING_INTERNER;
use crate::mir::builders::FunctionBodyKind;
use crate::mir::types::checked_declaration::CheckedDeclaration;

impl<'ctx, 'a> CodeGenerator<'ctx, 'a> {
    pub fn declare_functions(&mut self) {
        for (decl_id, decl) in &self.program.declarations {
            if let CheckedDeclaration::Function(f) = decl {
                let name = STRING_INTERNER.resolve(f.identifier.name);

                let ret_ty = self.get_any_type(f.return_type.id);
                let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();

                for param in &f.params {
                    param_types.push(self.get_basic_type(param.ty.id).into());
                }

                let fn_type = if ret_ty.is_void_type() {
                    self.context.void_type().fn_type(&param_types, false)
                } else {
                    self.get_basic_type(f.return_type.id)
                        .fn_type(&param_types, false)
                };

                let linkage = if f.is_exported || name == "main" {
                    Some(Linkage::External)
                } else {
                    Some(Linkage::Private)
                };

                let fn_val = self.module.add_function(&name, fn_type, linkage);
                self.functions.insert(*decl_id, fn_val);
            }
        }
    }

    pub fn define_functions(&mut self) {
        for (decl_id, decl) in &self.program.declarations {
            if let CheckedDeclaration::Function(f) = decl {
                if let FunctionBodyKind::Internal(cfg) = &f.body {
                    let fn_val = self.functions[decl_id];

                    for &block_id in cfg.blocks.keys() {
                        let block_name = format!("block_{}", block_id.0);
                        let llvm_block =
                            self.context.append_basic_block(fn_val, &block_name);
                        self.blocks.insert(block_id, llvm_block);
                    }

                    for (i, param) in f.params.iter().enumerate() {
                        if let Some(val_id) = param.value_id {
                            let llvm_param = fn_val.get_nth_param(i as u32).unwrap();

                            let param_name =
                                STRING_INTERNER.resolve(param.identifier.name);
                            llvm_param.set_name(&param_name);

                            self.values.insert(val_id, llvm_param);
                        }
                    }

                    for (block_id, bb) in &cfg.blocks {
                        let llvm_block = self.blocks[block_id];
                        self.builder.position_at_end(llvm_block);

                        for instr in &bb.instructions {
                            self.emit_instruction(instr);
                        }

                        if let Some(terminator) = &bb.terminator {
                            self.emit_terminator(terminator);
                        }
                    }
                }
            }
        }
    }
}
