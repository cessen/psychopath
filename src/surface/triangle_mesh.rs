#![allow(dead_code)]

use math::{Point, Normal, Matrix4x4};
use ray::Ray;
use triangle;
use bbox::BBox;
use bvh::BVH;

use super::{Surface, SurfaceIntersection};

pub struct TriangleMesh {
    time_samples: usize,
    geo: Vec<(Point, Point, Point)>,
    accel: BVH,
}

impl TriangleMesh {
    pub fn from_triangles(time_samples: usize,
                          mut triangles: Vec<(Point, Point, Point)>)
                          -> TriangleMesh {
        assert!(triangles.len() % time_samples == 0);
        // let mut indices: Vec<usize> = (0 .. (triangles.len() / time_samples)).collect();

        let accel = BVH::from_objects(&mut triangles[..], 3, |tri, bounds| {
            // for tri in &triangles[i..(i+time_samples)] {
            let minimum = tri.0.min(tri.1.min(tri.2));
            let maximum = tri.0.max(tri.1.max(tri.2));
            bounds.push(BBox::from_points(minimum, maximum));
            // }
        });

        TriangleMesh {
            time_samples: time_samples,
            geo: triangles,
            accel: accel,
        }
    }
}


impl Surface for TriangleMesh {
    fn intersect_rays(&self, rays: &mut [Ray], isects: &mut [SurfaceIntersection]) {
        self.accel.traverse(&mut rays[..], &self.geo, |tri, rs| {
            for r in rs {
                if let Some((t, tri_u, tri_v)) = triangle::intersect_ray(r, *tri) {
                    if t < r.max_t {
                        isects[r.id as usize] = SurfaceIntersection::Hit {
                            t: t,
                            pos: r.orig + (r.dir * t),
                            nor: Normal::new(0.0, 0.0, 0.0), // TODO
                            space: Matrix4x4::new(), // TODO
                            uv: (tri_u, tri_v),
                        };
                        r.max_t = t;
                    }
                }
            }
        });
    }
}
