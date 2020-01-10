#![allow(dead_code)]

use std::{io::BufRead, result::Result};

use nom::{sequence::tuple, IResult};

use kioku::Arena;

use data_tree::{reader::DataTreeReader, Event};

use crate::{
    math::{Normal, Point},
    surface::triangle_mesh::TriangleMesh,
};

use super::{
    basics::{ws_f32, ws_usize},
    psy::PsyParseError,
};

// pub struct TriangleMesh {
//    time_samples: usize,
//    geo: Vec<(Point, Point, Point)>,
//    indices: Vec<usize>,
//    accel: BVH,
// }

pub fn parse_mesh_surface<'a>(
    arena: &'a Arena,
    events: &mut DataTreeReader<impl BufRead>,
) -> Result<TriangleMesh<'a>, PsyParseError> {
    let mut verts = Vec::new(); // Vec of vecs, one for each time sample
    let mut normals = Vec::new(); // Vec of vecs, on for each time sample
    let mut face_vert_counts = Vec::new();
    let mut face_vert_indices = Vec::new();

    loop {
        match events.next_event()? {
            Event::Leaf {
                type_name: "SurfaceShaderBind",
                ..
            } => {
                // TODO
            }

            Event::Leaf {
                type_name: "Vertices",
                contents,
                ..
            } => {
                // Collect verts for this time sample
                let mut text = contents;
                let mut tverts = Vec::new();
                while let IResult::Ok((remaining, vert)) = tuple((ws_f32, ws_f32, ws_f32))(text) {
                    text = remaining;

                    tverts.push(Point::new(vert.0, vert.1, vert.2));
                }
                verts.push(tverts);
            }

            Event::Leaf {
                type_name: "Normals",
                contents,
                ..
            } => {
                // Collect normals for this time sample
                let mut text = contents;
                let mut tnormals = Vec::new();
                while let IResult::Ok((remaining, nor)) = tuple((ws_f32, ws_f32, ws_f32))(text) {
                    text = remaining;

                    tnormals.push(Normal::new(nor.0, nor.1, nor.2).normalized());
                }
                normals.push(tnormals);
            }

            Event::Leaf {
                type_name: "FaceVertCounts",
                contents,
                byte_offset,
            } => {
                if !face_vert_counts.is_empty() {
                    return Err(PsyParseError::WrongNodeCount(
                        byte_offset,
                        "Meshes can only have one FaceVertCounts section.",
                    ));
                }
                let mut text = contents;
                while let IResult::Ok((remaining, count)) = ws_usize(text) {
                    text = remaining;
                    face_vert_counts.push(count);
                }
            }

            Event::Leaf {
                type_name: "FaceVertIndices",
                contents,
                byte_offset,
            } => {
                if !face_vert_indices.is_empty() {
                    return Err(PsyParseError::WrongNodeCount(
                        byte_offset,
                        "Meshes can only have one FaceVertIndices section.",
                    ));
                }
                let mut text = contents;
                while let IResult::Ok((remaining, index)) = ws_usize(text) {
                    text = remaining;
                    face_vert_indices.push(index);
                }
            }

            Event::InnerClose { .. } => {
                break;
            }

            _ => {
                todo!(); // Return error.
            }
        }
    }

    // Validation: make sure all time samples have same vert count.
    let vert_count = verts[0].len();
    for vs in &verts {
        assert_eq!(vert_count, vs.len());
    }

    // Validation: make sure normal's time samples and vert count match
    // the vertices.
    if !normals.is_empty() {
        assert_eq!(normals.len(), verts.len());
        for ns in &normals {
            assert_eq!(vert_count, ns.len());
        }
    }

    // Validation: make sure we have any mesh data.
    if verts.is_empty() || face_vert_counts.is_empty() || face_vert_indices.is_empty() {
        todo!("Meshes must have at least one non-empty of each of the following sections: Vertices, FaceVertCounts, FaceVertIndices.");
        // Return an error.
    }

    // Build triangle mesh
    let mut tri_vert_indices = Vec::new();
    let mut ii = 0;
    for fvc in &face_vert_counts {
        if *fvc >= 3 {
            // Store the polygon, split up into triangles if >3 verts
            let v1 = ii;
            for vi in 0..(fvc - 2) {
                tri_vert_indices.push((
                    face_vert_indices[v1],
                    face_vert_indices[v1 + vi + 1],
                    face_vert_indices[v1 + vi + 2],
                ));
            }
        } else {
            // TODO: proper error
            panic!("Cannot handle polygons with less than three vertices.");
        }

        ii += *fvc;
    }

    Ok(TriangleMesh::from_verts_and_indices(
        arena,
        &verts,
        &if normals.is_empty() {
            None
        } else {
            Some(normals)
        },
        &tri_vert_indices,
    ))
}
