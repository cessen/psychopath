#![allow(dead_code)]

use std::result::Result;

use nom::IResult;

use super::DataTree;
use super::basics::{ws_usize, ws_f32};
use super::psy::PsyParseError;

use surface::triangle_mesh::TriangleMesh;
use math::Point;

// pub struct TriangleMesh {
//    time_samples: usize,
//    geo: Vec<(Point, Point, Point)>,
//    indices: Vec<usize>,
//    accel: BVH,
// }

pub fn parse_mesh_surface(tree: &DataTree) -> Result<TriangleMesh, PsyParseError> {
    let mut verts = Vec::new();
    let mut face_vert_counts = Vec::new();
    let mut face_vert_indices = Vec::new();

    // TODO: make sure there are the right number of various children,
    // and other validation.

    // Get verts
    // TODO: store vert count for a single round and make sure all rounds
    // have the same count.
    let mut time_samples = 0;
    for (_, text) in tree.iter_leaf_children_with_type("Vertices") {
        let mut raw_text = text.trim().as_bytes();

        while let IResult::Done(remaining, vert) = closure!(tuple!(ws_f32,
                                                                   ws_f32,
                                                                   ws_f32))(raw_text) {
            raw_text = remaining;

            verts.push(Point::new(vert.0, vert.1, vert.2));
        }

        time_samples += 1;
    }

    // Get face vert counts
    if let Some((_, text)) = tree.iter_leaf_children_with_type("FaceVertCounts").nth(0) {
        let mut raw_text = text.trim().as_bytes();

        while let IResult::Done(remaining, count) = ws_usize(raw_text) {
            raw_text = remaining;

            face_vert_counts.push(count);
        }
    }

    // Get face vert indices
    if let Some((_, text)) = tree.iter_leaf_children_with_type("FaceVertIndices").nth(0) {
        let mut raw_text = text.trim().as_bytes();

        while let IResult::Done(remaining, index) = ws_usize(raw_text) {
            raw_text = remaining;

            face_vert_indices.push(index);
        }
    }

    // Build triangle mesh
    // TODO: time samples
    let mut triangles = Vec::new();
    let mut ii = 0;
    for fvc in face_vert_counts.iter() {
        if *fvc >= 3 {
            let v1 = ii;
            for vi in 0..(fvc - 2) {
                triangles.push((verts[face_vert_indices[v1]],
                                verts[face_vert_indices[v1 + vi + 1]],
                                verts[face_vert_indices[v1 + vi + 2]]));
            }
        } else {
            // TODO: proper error
            panic!();
        }

        ii += *fvc;
    }

    Ok(TriangleMesh::from_triangles(time_samples, triangles))
}
