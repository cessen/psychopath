#![allow(dead_code)]

use lerp::{lerp, lerp_slice_with};
use math::{Point, Normal, Matrix4x4};
use ray::Ray;
use triangle;
use bbox::BBox;
use bvh::BVH;

use super::{Surface, SurfaceIntersection};

#[derive(Debug)]
pub struct TriangleMesh {
    time_samples: usize,
    geo: Vec<(Point, Point, Point)>,
    indices: Vec<usize>,
    accel: BVH,
}

impl TriangleMesh {
    pub fn from_triangles(time_samples: usize,
                          triangles: Vec<(Point, Point, Point)>)
                          -> TriangleMesh {
        assert!(triangles.len() % time_samples == 0);

        let mut indices: Vec<usize> = (0..(triangles.len() / time_samples))
                                          .map(|n| n * time_samples)
                                          .collect();

        let bounds = {
            let mut bounds = Vec::new();
            for tri in triangles.iter() {
                let minimum = tri.0.min(tri.1.min(tri.2));
                let maximum = tri.0.max(tri.1.max(tri.2));
                bounds.push(BBox::from_points(minimum, maximum));
            }
            bounds
        };

        let accel = BVH::from_objects(&mut indices[..],
                                      3,
                                      |tri_i| &bounds[*tri_i..(*tri_i + time_samples)]);

        TriangleMesh {
            time_samples: time_samples,
            geo: triangles,
            indices: indices,
            accel: accel,
        }
    }
}


impl Surface for TriangleMesh {
    fn intersect_rays(&self, rays: &mut [Ray], isects: &mut [SurfaceIntersection]) {
        self.accel.traverse(&mut rays[..], &self.indices, |tri_i, rs| {
            for r in rs {
                let tri = lerp_slice_with(&self.geo[*tri_i..(*tri_i + self.time_samples)],
                                          r.time,
                                          |a, b, t| {
                                              (lerp(a.0, b.0, t),
                                               lerp(a.1, b.1, t),
                                               lerp(a.2, b.2, t))
                                          });
                if let Some((t, tri_u, tri_v)) = triangle::intersect_ray(r, tri) {
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
