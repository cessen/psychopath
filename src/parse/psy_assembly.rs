#![allow(dead_code)]

use std::result::Result;

use kioku::Arena;

use crate::scene::{Assembly, Object, ObjectData};

use super::{
    psy::{parse_matrix, PsyParseError},
    psy_light::{parse_rectangle_light, parse_sphere_light},
    psy_mesh_surface::parse_mesh_surface,
    DataTree,
};

pub fn parse_assembly<'a>(
    arena: &'a Arena,
    tree: &'a DataTree,
) -> Result<Assembly<'a>, PsyParseError> {
    if !tree.is_internal() {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }

    let mut assembly = Assembly::new();
    for object in tree.iter_children() {
        if object.type_name() == "Object" {
            // Get object identifier.
            let object_ident = if let Some(ident) = object.ident() {
                ident
            } else {
                return Err(PsyParseError::ExpectedIdent(
                    object.byte_offset(),
                    "\'Object\' types must have an identifier, but the identifier is missing.",
                ));
            };

            // Collect instances.
            let mut instance_xform_idxs = Vec::new();
            for instance in object.iter_children_with_type("Instance") {
                if !instance.is_internal() {
                    // TODO: error.
                }

                let xforms_start_idx = assembly.xforms.len();
                for (_, contents, _) in instance.iter_leaf_children_with_type("Transform") {
                    assembly.xforms.push(parse_matrix(contents)?);
                }
                instance_xform_idxs.push(xforms_start_idx..assembly.xforms.len());
            }

            // Get object data.
            let object_data = {
                let obj_data_tree = {
                    if object
                        .iter_children()
                        .filter(|d| d.type_name() != "Instance")
                        .count()
                        != 1
                    {
                        // TODO: error.
                    }
                    object
                        .iter_children()
                        .filter(|d| d.type_name() != "Instance")
                        .nth(0)
                        .unwrap()
                };

                match obj_data_tree.type_name() {
                    // Sub-Assembly
                    "Assembly" => {
                        ObjectData::Assembly(Box::new(parse_assembly(arena, obj_data_tree)?))
                    }

                    "MeshSurface" => {
                        ObjectData::Surface(Box::new(parse_mesh_surface(arena, obj_data_tree)?))
                    }

                    "SphereLight" => {
                        ObjectData::Light(Box::new(parse_sphere_light(arena, obj_data_tree)?))
                    }

                    "RectangleLight" => {
                        ObjectData::Light(Box::new(parse_rectangle_light(arena, obj_data_tree)?))
                    }

                    _ => {
                        return Err(PsyParseError::UnknownVariant(
                            tree.byte_offset(),
                            "Unknown data type for Object.",
                        ));
                    }
                }
            };

            assembly.objects.insert(
                object_ident.to_string(),
                Object {
                    data: object_data,
                    instance_xform_idxs: instance_xform_idxs,
                },
            );
        } else {
            // TODO: error.
        }
    }

    return Ok(assembly);
}
