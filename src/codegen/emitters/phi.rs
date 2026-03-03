use std::collections::HashMap;

use inkwell::values::PhiValue;

use crate::{
    codegen::CodeGenerator,
    hir::builders::{BasicBlock, ValueId},
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
        &self,
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
                    let incoming_bb = self
                        .fn_blocks
                        .get(&source.from)
                        .expect("Phi source block not found");

                    let src_ty = self
                        .program
                        .value_types
                        .get(&source.value)
                        .expect("Source type missing");

                    if src_ty != phi_ty {
                        panic!(
                            "INTERNAL COMPILER ERROR: Phi node type mismatch in Codegen.\n\
                             Phi ID: {:?}\n\
                             Phi Type: {:?}\n\
                             Source Value: {:?}\n\
                             Source Type: {:?}\n\
                             Source Block: {:?}\n\
                             HIR should have inserted explicit casts or split edges.",
                            phi_val_id, phi_ty, source.value, src_ty, source.from
                        );
                    }

                    phi.add_incoming(&[(&incoming_val, *incoming_bb)]);
                }
            }
        }
    }
}
