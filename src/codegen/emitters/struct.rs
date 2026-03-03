use inkwell::values::BasicValue;

use crate::{
    codegen::CodeGenerator,
    hir::{instructions::StructInstr, types::checked_type::Type},
};

impl<'ctx> CodeGenerator<'ctx> {
    pub fn emit_struct(&mut self, instr: &StructInstr) {
        match instr {
            StructInstr::Construct { dest, fields } => {
                let struct_ty = self
                    .program
                    .value_types
                    .get(dest)
                    .expect("INTERNAL COMPILER ERROR: Struct type missing");

                let layout = if let Type::Struct(def_fields) = struct_ty {
                    self.get_struct_layout(def_fields)
                } else {
                    panic!("INTERNAL COMPILER ERROR: Construct target is not a struct");
                };

                let ptr = self
                    .builder
                    .build_malloc(layout, "struct_alloc")
                    .expect("Failed to emit malloc");

                if let Type::Struct(def_fields) = struct_ty {
                    for (field_name, val_id) in fields {
                        let (index, field_def) = def_fields
                            .iter()
                            .enumerate()
                            .find(|(_, f)| f.identifier.name == *field_name)
                            .expect("INTERNAL COMPILER ERROR: Field not found in struct definition");

                        let val = self.get_val_strict(*val_id);
                        let val_ty =
                            self.program.value_types.get(val_id).expect(
                                "INTERNAL COMPILER ERROR: Field value type missing",
                            );

                        if val_ty != &field_def.ty {
                            panic!(
                                "INTERNAL COMPILER ERROR: Struct field type mismatch.\n\
                                 Field: {:?}\n\
                                 Expected: {:?}\n\
                                 Got: {:?}",
                                field_name, field_def.ty, val_ty
                            );
                        }

                        let field_ptr = self
                            .builder
                            .build_struct_gep(layout, ptr, index as u32, "field_ptr")
                            .unwrap();

                        self.builder.build_store(field_ptr, val).unwrap();
                    }
                }

                self.fn_values.insert(*dest, ptr.as_basic_value_enum());
            }

            StructInstr::ReadField { dest, base, field } => {
                let base_ptr = self.get_val_strict(*base).into_pointer_value();
                let base_ty = self.program.value_types.get(base).unwrap();

                if let Type::Struct(def_fields) = base_ty {
                    let layout = self.get_struct_layout(def_fields);

                    let (index, field_def) = def_fields
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.identifier.name == *field)
                        .expect("INTERNAL COMPILER ERROR: Field not found");

                    let field_ptr = self
                        .builder
                        .build_struct_gep(layout, base_ptr, index as u32, "field_gep")
                        .unwrap();

                    let res = self
                        .builder
                        .build_load(
                            self.lower_type(&field_def.ty).unwrap(),
                            field_ptr,
                            "field_val",
                        )
                        .unwrap();

                    self.fn_values.insert(*dest, res);
                } else {
                    panic!("INTERNAL COMPILER ERROR: ReadField base is not a struct");
                }
            }

            StructInstr::UpdateField {
                dest,
                base,
                field,
                value,
            } => {
                let base_ptr = self.get_val_strict(*base).into_pointer_value();
                let base_ty = self.program.value_types.get(base).unwrap();

                let new_val = self.get_val_strict(*value);
                let new_val_ty = self.program.value_types.get(value).unwrap();

                if let Type::Struct(def_fields) = base_ty {
                    let layout = self.get_struct_layout(def_fields);

                    let (index, field_def) = def_fields
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.identifier.name == *field)
                        .expect("INTERNAL COMPILER ERROR: Field not found");

                    if new_val_ty != &field_def.ty {
                        panic!(
                            "INTERNAL COMPILER ERROR: UpdateField type mismatch.\n\
                             Field: {:?}\n\
                             Expected: {:?}\n\
                             Got: {:?}",
                            field, field_def.ty, new_val_ty
                        );
                    }

                    let field_ptr = self
                        .builder
                        .build_struct_gep(layout, base_ptr, index as u32, "update_gep")
                        .unwrap();

                    self.builder.build_store(field_ptr, new_val).unwrap();

                    self.fn_values.insert(*dest, base_ptr.as_basic_value_enum());
                } else {
                    panic!("INTERNAL COMPILER ERROR: UpdateField base is not a struct");
                }
            }
        }
    }
}
