#![allow(dead_code)]

use std::result::Result;

use nom::{sequence::tuple, IResult};

use kioku::Arena;

use data_tree::{
    reader::{DataTreeReader, ReaderError},
    Event,
};

use crate::{
    math::{Normal, Point},
    surface::triangle_mesh::TriangleMesh,
};

use super::{
    basics::{ws_f32, ws_usize},
    psy::PsyParseError,
    DataTree,
};

// pub struct TriangleMesh {
//    time_samples: usize,
//    geo: Vec<(Point, Point, Point)>,
//    indices: Vec<usize>,
//    accel: BVH,
// }

pub fn parse_mesh_surface<'a>(
    arena: &'a Arena,
    events: &mut DataTreeReader,
    ident: Option<&str>,
) -> Result<TriangleMesh<'a>, PsyParseError> {
    let mut verts = Vec::new(); // Vec of vecs, one for each time sample
    let mut normals = Vec::new(); // Vec of vecs, on for each time sample
    let mut face_vert_counts = Vec::new();
    let mut face_vert_indices = Vec::new();

    // TODO: make sure there are the right number of various children,
    // and other validation.

    // Get verts
    for (_, mut text, _) in tree.iter_leaf_children_with_type("Vertices") {
        // Collect verts for this time sample
        let mut tverts = Vec::new();
        while let IResult::Ok((remaining, vert)) = tuple((ws_f32, ws_f32, ws_f32))(text) {
            text = remaining;

            tverts.push(Point::new(vert.0, vert.1, vert.2));
        }
        verts.push(tverts);
    }

    // Make sure all time samples have same vert count
    let vert_count = verts[0].len();
    for vs in &verts {
        assert_eq!(vert_count, vs.len());
    }

    // Get normals, if they exist
    for (_, mut text, _) in tree.iter_leaf_children_with_type("Normals") {
        // Collect normals for this time sample
        let mut tnormals = Vec::new();
        while let IResult::Ok((remaining, nor)) = tuple((ws_f32, ws_f32, ws_f32))(text) {
            text = remaining;

            tnormals.push(Normal::new(nor.0, nor.1, nor.2).normalized());
        }
        normals.push(tnormals);
    }

    // Make sure normal's time samples and vert count match the vertices
    if !normals.is_empty() {
        assert_eq!(normals.len(), verts.len());
        for ns in &normals {
            assert_eq!(vert_count, ns.len());
        }
    }

    // Get face vert counts
    if let Some((_, mut text, _)) = tree.iter_leaf_children_with_type("FaceVertCounts").nth(0) {
        while let IResult::Ok((remaining, count)) = ws_usize(text) {
            text = remaining;

            face_vert_counts.push(count);
        }
    }

    // Get face vert indices
    if let Some((_, mut text, _)) = tree.iter_leaf_children_with_type("FaceVertIndices").nth(0) {
        while let IResult::Ok((remaining, index)) = ws_usize(text) {
            text = remaining;

            face_vert_indices.push(index);
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
