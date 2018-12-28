#![allow(dead_code)]

use std::result::Result;

use nom::{call, closure, tuple, tuple_parser, IResult};

use mem_arena::MemArena;

use crate::{
    color::{rec709_e_to_xyz, Color},
    shading::{SimpleSurfaceShader, SurfaceShader},
};

use super::{basics::ws_f32, psy::PsyParseError, DataTree};

// pub struct TriangleMesh {
//    time_samples: usize,
//    geo: Vec<(Point, Point, Point)>,
//    indices: Vec<usize>,
//    accel: BVH,
// }

pub fn parse_surface_shader<'a>(
    arena: &'a MemArena,
    tree: &'a DataTree,
) -> Result<&'a SurfaceShader, PsyParseError> {
    let type_name = if let Some((_, text, _)) = tree.iter_leaf_children_with_type("Type").nth(0) {
        text.trim()
    } else {
        return Err(PsyParseError::MissingNode(
            tree.byte_offset(),
            "Expected a Type field in SurfaceShader.",
        ));
    };

    let shader = match type_name {
        "Lambert" => {
            let color = if let Some((_, contents, byte_offset)) =
                tree.iter_leaf_children_with_type("Color").nth(0)
            {
                if let IResult::Done(_, color) =
                    closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.as_bytes())
                {
                    // TODO: handle color space conversions properly.
                    // Probably will need a special color type with its
                    // own parser...?
                    Color::new_xyz(rec709_e_to_xyz(color))
                } else {
                    // Found color, but its contents is not in the right format
                    return Err(PsyParseError::UnknownError(byte_offset));
                }
            } else {
                return Err(PsyParseError::MissingNode(
                    tree.byte_offset(),
                    "Expected a Color field in Lambert SurfaceShader.",
                ));
            };

            arena.alloc(SimpleSurfaceShader::Lambert { color: color })
        }

        "GGX" => {
            // Color
            let color = if let Some((_, contents, byte_offset)) =
                tree.iter_leaf_children_with_type("Color").nth(0)
            {
                if let IResult::Done(_, color) =
                    closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.as_bytes())
                {
                    // TODO: handle color space conversions properly.
                    // Probably will need a special color type with its
                    // own parser...?
                    Color::new_xyz(rec709_e_to_xyz(color))
                } else {
                    // Found color, but its contents is not in the right format
                    return Err(PsyParseError::UnknownError(byte_offset));
                }
            } else {
                return Err(PsyParseError::MissingNode(
                    tree.byte_offset(),
                    "Expected a Color field in GTR SurfaceShader.",
                ));
            };

            // Roughness
            let roughness = if let Some((_, contents, byte_offset)) =
                tree.iter_leaf_children_with_type("Roughness").nth(0)
            {
                if let IResult::Done(_, roughness) = ws_f32(contents.as_bytes()) {
                    roughness
                } else {
                    return Err(PsyParseError::UnknownError(byte_offset));
                }
            } else {
                return Err(PsyParseError::MissingNode(
                    tree.byte_offset(),
                    "Expected a Roughness field in GTR SurfaceShader.",
                ));
            };

            // Fresnel
            let fresnel = if let Some((_, contents, byte_offset)) =
                tree.iter_leaf_children_with_type("Fresnel").nth(0)
            {
                if let IResult::Done(_, fresnel) = ws_f32(contents.as_bytes()) {
                    fresnel
                } else {
                    return Err(PsyParseError::UnknownError(byte_offset));
                }
            } else {
                return Err(PsyParseError::MissingNode(
                    tree.byte_offset(),
                    "Expected a Fresnel field in GTR SurfaceShader.",
                ));
            };

            arena.alloc(SimpleSurfaceShader::GGX {
                color: color,
                roughness: roughness,
                fresnel: fresnel,
            })
        }

        "Emit" => {
            let color = if let Some((_, contents, byte_offset)) =
                tree.iter_leaf_children_with_type("Color").nth(0)
            {
                if let IResult::Done(_, color) =
                    closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.as_bytes())
                {
                    // TODO: handle color space conversions properly.
                    // Probably will need a special color type with its
                    // own parser...?
                    Color::new_xyz(rec709_e_to_xyz(color))
                } else {
                    // Found color, but its contents is not in the right format
                    return Err(PsyParseError::UnknownError(byte_offset));
                }
            } else {
                return Err(PsyParseError::MissingNode(
                    tree.byte_offset(),
                    "Expected a Color field in Emit SurfaceShader.",
                ));
            };

            arena.alloc(SimpleSurfaceShader::Emit { color: color })
        }

        _ => unimplemented!(),
    };

    Ok(shader)
}
