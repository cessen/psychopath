#![allow(dead_code)]

use std::result::Result;

use nom::IResult;

use mem_arena::MemArena;

use math::Point;
use surface::triangle_mesh::TriangleMesh;

use super::basics::{ws_usize, ws_f32};
use super::DataTree;
use super::psy::PsyParseError;


// pub struct TriangleMesh {
//    time_samples: usize,
//    geo: Vec<(Point, Point, Point)>,
//    indices: Vec<usize>,
//    accel: BVH,
// }

pub fn parse_mesh_surface<'a>(arena: &'a MemArena,
                              tree: &'a DataTree)
                              -> Result<TriangleMesh<'a>, PsyParseError> {
    let mut verts = Vec::new();
    let mut face_vert_counts = Vec::new();
    let mut face_vert_indices = Vec::new();

    // TODO: make sure there are the right number of various children,
    // and other validation.

    // Get verts
    let mut time_samples = 0;
    let mut first_vert_count = None;
    for (_, text) in tree.iter_leaf_children_with_type("Vertices") {
        let mut raw_text = text.trim().as_bytes();

        // Collect verts for this time sample
        let mut vert_count = 0;
        while let IResult::Done(remaining, vert) =
            closure!(tuple!(ws_f32, ws_f32, ws_f32))(raw_text) {
            raw_text = remaining;

            verts.push(Point::new(vert.0, vert.1, vert.2));
            vert_count += 1;
        }

        // Make sure all time samples have same vert count
        if let Some(fvc) = first_vert_count {
            assert!(vert_count == fvc);
        } else {
            first_vert_count = Some(vert_count);
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
    let mut triangles = Vec::new();
    let vert_count = first_vert_count.unwrap();
    let mut ii = 0;
    for fvc in face_vert_counts.iter() {
        if *fvc >= 3 {
            // Store the polygon, split up into triangles if >3 verts
            let v1 = ii;
            for vi in 0..(fvc - 2) {
                // Store all the time samples of each triangle contiguously
                for time_sample in 0..time_samples {
                    let start_vi = vert_count * time_sample;
                    triangles.push((verts[start_vi + face_vert_indices[v1]],
                                    verts[start_vi + face_vert_indices[v1 + vi + 1]],
                                    verts[start_vi + face_vert_indices[v1 + vi + 2]]));
                }
            }
        } else {
            // TODO: proper error
            panic!("Cannot handle polygons with less than three vertices.");
        }

        ii += *fvc;
    }

    Ok(TriangleMesh::from_triangles(arena, time_samples, triangles))
}
