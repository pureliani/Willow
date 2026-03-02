use std::collections::HashMap;

use inkwell::values::PhiValue;

use crate::{
    codegen::CodeGenerator,
    globals::STRING_INTERNER,
    hir::{
        builders::{BasicBlock, BasicBlockId, ValueId},
        instructions::Terminator,
    },
};

impl<'ctx> CodeGenerator<'ctx> {
    pub fn create_phis_for_block(
        &mut self,
        block: &BasicBlock,
        phi_nodes: &mut HashMap<ValueId, PhiValue<'ctx>>,
    ) {
        let llvm_bb = self.fn_blocks.get(&block.id).unwrap();
        self.builder.position_at_end(*llvm_bb);

        for phi_val_id in block.phis.keys() {
            let phi_ty = self
                .program
                .value_types
                .get(phi_val_id)
                .expect("INTERNAL COMPILER ERROR: Phi type missing");

            if let Some(llvm_ty) = self.lower_type(phi_ty) {
                let phi_name = format!("phi_{}", phi_val_id.0);
                let phi = self.builder.build_phi(llvm_ty, &phi_name).unwrap();

                self.fn_values.insert(*phi_val_id, phi.as_basic_value());
                phi_nodes.insert(*phi_val_id, phi);
            }
        }
    }

    pub fn resolve_phis_for_block(
        &mut self,
        block: &BasicBlock,
        phi_nodes: &HashMap<ValueId, PhiValue<'ctx>>,
    ) {
        for (phi_val_id, sources) in &block.phis {
            if let Some(phi) = phi_nodes.get(phi_val_id) {
                let phi_ty = self
                    .program
                    .value_types
                    .get(phi_val_id)
                    .expect("Phi type missing");

                for source in sources {
                    let incoming_val = self.get_val_strict(source.value);
                    let mut incoming_bb = *self
                        .fn_blocks
                        .get(&source.from)
                        .expect("Phi source block not found");

                    let src_ty = self
                        .program
                        .value_types
                        .get(&source.value)
                        .expect("Source type missing");

                    let final_val = if src_ty != phi_ty {
                        let fn_name = STRING_INTERNER
                            .resolve(self.current_fn.unwrap().identifier.name);
                        let current_fn = self.module.get_function(&fn_name).unwrap();

                        let split_bb =
                            self.context.append_basic_block(current_fn, "edge_split");

                        self.builder.position_at_end(split_bb);
                        let casted_val = self.emit_cast(incoming_val, src_ty, phi_ty);

                        let phi_bb = *self.fn_blocks.get(&block.id).unwrap();
                        self.builder.build_unconditional_branch(phi_bb).unwrap();

                        let pred_hir_block =
                            self.current_fn.unwrap().blocks.get(&source.from).unwrap();

                        if let Some(term) = &pred_hir_block.terminator {
                            self.redirect_terminator(
                                incoming_bb,
                                term,
                                block.id,
                                split_bb,
                            );
                        }

                        incoming_bb = split_bb;
                        casted_val
                    } else {
                        incoming_val
                    };

                    phi.add_incoming(&[(&final_val, incoming_bb)]);
                }
            }
        }
    }

    fn redirect_terminator(
        &self,
        llvm_bb: inkwell::basic_block::BasicBlock<'ctx>,
        hir_term: &Terminator,
        old_target_id: BasicBlockId,
        new_target_bb: inkwell::basic_block::BasicBlock<'ctx>,
    ) {
        if let Some(term_instr) = llvm_bb.get_terminator() {
            term_instr.erase_from_basic_block();
        }

        self.builder.position_at_end(llvm_bb);

        match hir_term {
            Terminator::Jump { target } => {
                let dest = if *target == old_target_id {
                    new_target_bb
                } else {
                    *self.fn_blocks.get(target).unwrap()
                };
                self.builder.build_unconditional_branch(dest).unwrap();
            }
            Terminator::CondJump {
                condition,
                true_target,
                false_target,
            } => {
                let cond = self.get_val_strict(*condition).into_int_value();

                let t_dest = if *true_target == old_target_id {
                    new_target_bb
                } else {
                    *self.fn_blocks.get(true_target).unwrap()
                };

                let f_dest = if *false_target == old_target_id {
                    new_target_bb
                } else {
                    *self.fn_blocks.get(false_target).unwrap()
                };

                self.builder
                    .build_conditional_branch(cond, t_dest, f_dest)
                    .unwrap();
            }
            Terminator::Return { .. } => {
                panic!("INTERNAL COMPILER ERROR: Phi source block cannot end with Return")
            }
        }
    }
}
