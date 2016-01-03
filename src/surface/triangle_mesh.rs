#![allow(dead_code)]

use math::{Point, Normal, Matrix4x4};
use ray::Ray;
use triangle;
use bbox::BBox;
use bvh::BVH;

use super::{Surface, SurfaceIntersection};

pub struct TriangleMesh {
    geo: Vec<(Point, Point, Point)>,
    accel: BVH,
}

impl TriangleMesh {
    pub fn from_triangles(mut triangles: Vec<(Point, Point, Point)>) -> TriangleMesh {
        let accel = BVH::from_objects(&mut triangles[..], 3, |tri| {
            let minimum = tri.0.min(tri.1.min(tri.2));
            let maximum = tri.0.max(tri.1.max(tri.2));
            BBox {
                min: minimum,
                max: maximum,
            }
        });

        TriangleMesh {
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
