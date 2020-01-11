#![allow(dead_code)]

use std::{io::BufRead, result::Result};

use nom::{combinator::all_consuming, IResult};

use kioku::Arena;

use data_tree::{DataTreeReader, Event};

use crate::shading::{SimpleSurfaceShader, SurfaceShader};

use super::{
    parse_utils::{ensure_close, ensure_subsections, ws_f32},
    psy::{parse_color, PsyError, PsyResult},
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
) -> PsyResult<Box<dyn SurfaceShader>> {
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
                parse_color(byte_offset, contents)?
            } else {
                return Err(PsyError::MissingNode(
                    events.byte_offset(),
                    "Expected a Color field in Lambert SurfaceShader.".into(),
                ));
            };

            // Close shader node.
            ensure_close(events)?;

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

            let valid_subsections = &[
                ("Color", true, (1).into()),
                ("Roughness", true, (1).into()),
                ("Fresnel", true, (1).into()),
            ];
            ensure_subsections(events, valid_subsections, |events| {
                match events.next_event()? {
                    // Color
                    Event::Leaf {
                        type_name: "Color",
                        contents,
                        byte_offset,
                    } => {
                        color = Some(parse_color(byte_offset, contents)?);
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
                            return Err(PsyError::IncorrectLeafData(
                                byte_offset,
                                "Roughness data isn't in the right format.  It \
                                 should contain a single floating point value."
                                    .into(),
                            ));
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
                            return Err(PsyError::IncorrectLeafData(
                                byte_offset,
                                "Fresnel data isn't in the right format.  It \
                                 should contain a single floating point value."
                                    .into(),
                            ));
                        }
                    }

                    _ => unreachable!(),
                }
                Ok(())
            })?;

            ensure_close(events)?;

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
                parse_color(byte_offset, contents)?
            } else {
                return Err(PsyError::MissingNode(
                    events.byte_offset(),
                    "Expected a Color field in Emit SurfaceShader.".into(),
                ));
            };

            // Close shader node.
            ensure_close(events)?;

            Box::new(SimpleSurfaceShader::Emit { color: color })
        }

        Event::Leaf {
            type_name: "Type",
            byte_offset,
            ..
        } => {
            return Err(PsyError::MissingNode(
                byte_offset,
                "Unknown SurfaceShader type.".into(),
            ));
        }
        _ => {
            todo!(); // Return error.
        }
    };

    Ok(shader)
}
