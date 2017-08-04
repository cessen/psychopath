#![allow(dead_code)]

use std::result::Result;

use nom::IResult;

use mem_arena::MemArena;

use color::{XYZ, rec709_e_to_xyz};
use shading::{SurfaceShader, SimpleSurfaceShader};

use super::basics::ws_f32;
use super::DataTree;
use super::psy::PsyParseError;


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
        "Emit" => unimplemented!(),
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
                    XYZ::from_tuple(rec709_e_to_xyz(color))
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
        "GTR" => unimplemented!(),
        _ => unimplemented!(),
    };

    Ok(shader)
}
