#![allow(dead_code)]

use std::{io::BufRead, result::Result};

use nom::{combinator::all_consuming, IResult};

use kioku::Arena;

use data_tree::{reader::DataTreeReader, Event};

use crate::shading::{SimpleSurfaceShader, SurfaceShader};

use super::{
    basics::ws_f32,
    psy::{parse_color, PsyParseError},
};

// pub struct TriangleMesh {
//    time_samples: usize,
//    geo: Vec<(Point, Point, Point)>,
//    indices: Vec<usize>,
//    accel: BVH,
// }

pub fn parse_surface_shader(
    _arena: &Arena,
    events: &mut DataTreeReader<impl BufRead>,
    _ident: Option<&str>,
) -> Result<Box<dyn SurfaceShader>, PsyParseError> {
    // Get shader type.
    let shader = match events.next_event()? {
        Event::Leaf {
            type_name: "Type",
            contents: "Lambert",
            ..
        } => {
            let color = if let Event::Leaf {
                type_name: "Color",
                contents,
                byte_offset,
            } = events.next_event()?
            {
                if let Ok(color) = parse_color(contents) {
                    color
                } else {
                    // Found color, but its contents is not in the right format
                    return Err(PsyParseError::UnknownError(byte_offset));
                }
            } else {
                return Err(PsyParseError::MissingNode(
                    events.byte_offset(),
                    "Expected a Color field in Lambert SurfaceShader.",
                ));
            };

            // Close shader node.
            if let Event::InnerClose { .. } = events.next_event()? {
                // Success, do nothing.
            } else {
                todo!(); // Return error.
            }

            Box::new(SimpleSurfaceShader::Lambert { color: color })
        }

        Event::Leaf {
            type_name: "Type",
            contents: "GGX",
            ..
        } => {
            let mut color = None;
            let mut roughness = None;
            let mut fresnel = None;

            loop {
                match events.next_event()? {
                    // Color
                    Event::Leaf {
                        type_name: "Color",
                        contents,
                        byte_offset,
                    } => {
                        if let Ok(col) = parse_color(contents) {
                            color = Some(col);
                        } else {
                            // Found color, but its contents is not in the right
                            // format.
                            return Err(PsyParseError::UnknownError(byte_offset));
                        }
                    }

                    // Roughness
                    Event::Leaf {
                        type_name: "Roughness",
                        contents,
                        byte_offset,
                    } => {
                        if let IResult::Ok((_, rgh)) = all_consuming(ws_f32)(contents) {
                            roughness = Some(rgh);
                        } else {
                            return Err(PsyParseError::UnknownError(byte_offset));
                        }
                    }

                    // Fresnel
                    Event::Leaf {
                        type_name: "Fresnel",
                        contents,
                        byte_offset,
                    } => {
                        if let IResult::Ok((_, frs)) = all_consuming(ws_f32)(contents) {
                            fresnel = Some(frs);
                        } else {
                            return Err(PsyParseError::UnknownError(byte_offset));
                        }
                    }

                    Event::InnerClose { .. } => {
                        break;
                    }

                    _ => {
                        todo!(); // Return an error.
                    }
                }
            }

            // Validation: make sure all fields are present.
            if color == None || roughness == None || fresnel == None {
                return Err(PsyParseError::MissingNode(
                    events.byte_offset(),
                    "GGX shader requires one of each field: Color, Roughness, Fresnel.",
                ));
            }

            Box::new(SimpleSurfaceShader::GGX {
                color: color.unwrap(),
                roughness: roughness.unwrap(),
                fresnel: fresnel.unwrap(),
            })
        }

        Event::Leaf {
            type_name: "Type",
            contents: "Emit",
            ..
        } => {
            let color = if let Event::Leaf {
                type_name: "Color",
                contents,
                byte_offset,
            } = events.next_event()?
            {
                if let Ok(color) = parse_color(contents) {
                    color
                } else {
                    // Found color, but its contents is not in the right format
                    return Err(PsyParseError::UnknownError(byte_offset));
                }
            } else {
                return Err(PsyParseError::MissingNode(
                    events.byte_offset(),
                    "Expected a Color field in Emit SurfaceShader.",
                ));
            };

            // Close shader node.
            if let Event::InnerClose { .. } = events.next_event()? {
                // Success, do nothing.
            } else {
                todo!(); // Return error.
            }

            Box::new(SimpleSurfaceShader::Emit { color: color })
        }

        Event::Leaf {
            type_name: "Type",
            byte_offset,
            ..
        } => {
            return Err(PsyParseError::MissingNode(
                byte_offset,
                "Unknown SurfaceShader type.",
            ));
        }
        _ => {
            todo!(); // Return error.
        }
    };

    Ok(shader)
}
