#![allow(dead_code)]

use std::{io::BufRead, result::Result};

use nom::{sequence::tuple, IResult};

use kioku::Arena;

use data_tree::{DataTreeReader, Event};

use crate::{
    math::{Normal, Point},
    surface::triangle_mesh::TriangleMesh,
};

use super::{
    parse_utils::{ensure_close, ensure_subsections, ws_f32, ws_usize},
    psy::{PsyError, PsyResult},
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
) -> PsyResult<TriangleMesh<'a>> {
    let mut verts = Vec::new(); // Vec of vecs, one for each time sample
    let mut normals = Vec::new(); // Vec of vecs, on for each time sample
    let mut face_vert_counts = Vec::new();
    let mut face_vert_indices = Vec::new();

    let valid_subsections = &[
        ("SurfaceShaderBind", true, (1).into()),
        ("Vertices", true, (1..).into()),
        ("Normals", true, (..).into()),
        ("FaceVertCounts", true, (1).into()),
        ("FaceVertIndices", true, (1).into()),
    ];
    ensure_subsections(events, valid_subsections, |events| {
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
                byte_offset,
            } => {
                // Collect verts for this time sample
                let mut text = contents;
                let mut tverts = Vec::new();
                while let IResult::Ok((remaining, vert)) = tuple((ws_f32, ws_f32, ws_f32))(text) {
                    text = remaining;

                    tverts.push(Point::new(vert.0, vert.1, vert.2));
                }
                if !text.is_empty() {
                    return Err(PsyError::IncorrectLeafData(
                        byte_offset,
                        "Vertices are not in the right format.  Each vertex \
                         must be specified by three decimal values."
                            .into(),
                    ));
                }
                verts.push(tverts);
            }

            Event::Leaf {
                type_name: "Normals",
                contents,
                byte_offset,
            } => {
                // Collect normals for this time sample
                let mut text = contents;
                let mut tnormals = Vec::new();
                while let IResult::Ok((remaining, nor)) = tuple((ws_f32, ws_f32, ws_f32))(text) {
                    text = remaining;

                    tnormals.push(Normal::new(nor.0, nor.1, nor.2).normalized());
                }
                if !text.is_empty() {
                    return Err(PsyError::IncorrectLeafData(
                        byte_offset,
                        "Normals are not in the right format.  Each normal \
                         must be specified by three decimal values."
                            .into(),
                    ));
                }
                normals.push(tnormals);
            }

            Event::Leaf {
                type_name: "FaceVertCounts",
                contents,
                byte_offset,
            } => {
                let mut text = contents;
                while let IResult::Ok((remaining, count)) = ws_usize(text) {
                    text = remaining;
                    face_vert_counts.push(count);
                }
                if !text.is_empty() {
                    return Err(PsyError::IncorrectLeafData(
                        byte_offset,
                        "FaceVertCounts are not in the right format.  Should be \
                         a simple list of space-separated integers."
                            .into(),
                    ));
                }
            }

            Event::Leaf {
                type_name: "FaceVertIndices",
                contents,
                byte_offset,
            } => {
                let mut text = contents;
                while let IResult::Ok((remaining, index)) = ws_usize(text) {
                    text = remaining;
                    face_vert_indices.push(index);
                }
                if !text.is_empty() {
                    return Err(PsyError::IncorrectLeafData(
                        byte_offset,
                        "FaceVertCounts are not in the right format.  Should be \
                         a simple list of space-separated integers."
                            .into(),
                    ));
                }
            }

            _ => unreachable!(),
        }
        Ok(())
    })?;

    ensure_close(events)?;

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
