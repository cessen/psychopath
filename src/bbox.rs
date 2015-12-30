#![allow(dead_code)]

use std;
use std::ops::BitOr;

use math::Point;
use lerp::{lerp, Lerp};
use ray::Ray;

const BBOX_MAXT_ADJUST: f32 = 1.00000024;

/// A 3D axis-aligned bounding box.
#[derive(Debug, Copy, Clone)]
pub struct BBox {
    pub min: Point,
    pub max: Point,
}

impl BBox {
    /// Creates a degenerate BBox with +infinity min and -infinity max.
    pub fn new() -> BBox {
        BBox {
            min: Point::new(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY),
            max: Point::new(std::f32::NEG_INFINITY,
                            std::f32::NEG_INFINITY,
                            std::f32::NEG_INFINITY),
        }
    }

    /// Creates a BBox with min as the minimum extent and max as the maximum
    /// extent.
    pub fn from_points(min: Point, max: Point) -> BBox {
        BBox {
            min: min,
            max: max,
        }
    }

    // Returns whether the given ray intersects with the bbox.
    pub fn intersect_ray(&self, ray: &Ray) -> bool {
        // Calculate slab intersections
        let t1 = (self.min.co - ray.orig.co) * ray.dir_inv.co;
        let t2 = (self.max.co - ray.orig.co) * ray.dir_inv.co;

        // Find the far and near intersection
        let hitt0 = (t1[0].min(t2[0]))
                        .max(t1[1].min(t2[1]))
                        .max(t1[2].min(t2[2]));
        let hitt1 = (t1[0].max(t2[0]))
                        .min(t1[1].max(t2[1]))
                        .min(t1[2].max(t2[2]));

        // Did we hit?
        return hitt0.max(0.0) <= hitt1.min(ray.max_t);
    }
}


/// Union of two BBoxes.
impl BitOr for BBox {
    type Output = BBox;

    fn bitor(self, rhs: BBox) -> BBox {
        BBox::from_points(Point { co: self.min.co.v_min(rhs.min.co) },
                          Point { co: self.max.co.v_max(rhs.max.co) })
    }
}


impl Lerp for BBox {
    fn lerp(self, other: BBox, alpha: f32) -> BBox {
        BBox {
            min: lerp(self.min, other.min, alpha),
            max: lerp(self.max, other.max, alpha),
        }
    }
}
