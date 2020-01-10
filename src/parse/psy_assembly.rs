#![allow(dead_code)]

use std::{io::BufRead, result::Result};

use kioku::Arena;

use data_tree::{DataTreeReader, Event};

use crate::scene::{Assembly, Object, ObjectData};

use super::{
    parse_utils::ensure_close,
    psy::{parse_matrix, PsyError, PsyResult},
    psy_light::{parse_rectangle_light, parse_sphere_light},
    psy_mesh_surface::parse_mesh_surface,
};

pub fn parse_assembly<'a>(
    arena: &'a Arena,
    events: &mut DataTreeReader<impl BufRead>,
) -> PsyResult<Assembly<'a>> {
    let mut assembly = Assembly::new();
    loop {
        match events.next_event()? {
            Event::InnerOpen {
                type_name: "Object",
                ident,
                byte_offset,
            } => {
                // Get object identifier.
                let object_ident = if let Some(id) = ident {
                    id.to_string()
                } else {
                    return Err(PsyError::ExpectedIdent(
                        byte_offset,
                        "\'Object\' types must have an identifier, but the identifier is missing."
                            .into(),
                    ));
                };

                // Collect instances.
                let mut instance_xform_idxs = Vec::new();
                while let Event::InnerOpen {
                    type_name: "Instance",
                    ..
                } = events.peek_event()?
                {
                    events.next_event();
                    let xforms_start_idx = assembly.xforms.len();
                    loop {
                        match events.next_event()? {
                            Event::Leaf {
                                type_name: "Transform",
                                contents,
                                ..
                            } => {
                                assembly.xforms.push(parse_matrix(contents)?);
                            }

                            Event::InnerClose { .. } => {
                                break;
                            }

                            _ => {
                                todo!("Instances can only contain Transforms.");
                                // Return an error.
                            }
                        }
                    }

                    instance_xform_idxs.push(xforms_start_idx..assembly.xforms.len());
                }

                // Get object data.
                let object_data = match events.next_event()? {
                    Event::InnerOpen {
                        type_name: "Assembly",
                        ..
                    } => ObjectData::Assembly(Box::new(parse_assembly(arena, events)?)),

                    Event::InnerOpen {
                        type_name: "MeshSurface",
                        ..
                    } => ObjectData::Surface(Box::new(parse_mesh_surface(arena, events)?)),

                    Event::InnerOpen {
                        type_name: "SphereLight",
                        ..
                    } => ObjectData::Light(Box::new(parse_sphere_light(arena, events)?)),

                    Event::InnerOpen {
                        type_name: "RectangleLight",
                        ..
                    } => ObjectData::Light(Box::new(parse_rectangle_light(arena, events)?)),

                    Event::InnerClose { byte_offset } => {
                        return Err(PsyError::MissingNode(
                            byte_offset,
                            "Object contains no object data.".into(),
                        ));
                    }

                    _ => {
                        return Err(PsyError::UnknownVariant(
                            byte_offset,
                            "Unknown data type for Object.".into(),
                        ));
                    }
                };

                // Close object node.
                ensure_close(events)?;

                assembly.objects.insert(
                    object_ident,
                    Object {
                        data: object_data,
                        instance_xform_idxs: instance_xform_idxs,
                    },
                );
            }

            Event::InnerClose { .. } => {
                break;
            }

            _ => {
                todo!(); // Return an error.
            }
        }
    }

    // if !tree.is_internal() {
    //     return Err(PsyError::UnknownError(tree.byte_offset()));
    // }

    // for object in tree.iter_children() {
    //     if object.type_name() == "Object" {

    //     } else {
    //         // TODO: error.
    //     }
    // }

    return Ok(assembly);
}
