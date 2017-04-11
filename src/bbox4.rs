#![allow(dead_code)]

use std;
use std::ops::{BitOr, BitOrAssign};

use bbox::BBox;
use float4::{Float4, Bool4, v_min, v_max};
use lerp::{lerp, Lerp};
use ray::AccelRay;


const BBOX_MAXT_ADJUST: f32 = 1.00000024;

/// A SIMD set of 4 3D axis-aligned bounding boxes.
#[derive(Debug, Copy, Clone)]
pub struct BBox4 {
    pub min: (Float4, Float4, Float4), // xs, ys, zs
    pub max: (Float4, Float4, Float4), // xs, ys, zs
}

impl BBox4 {
    /// Creates a degenerate BBox with +infinity min and -infinity max.
    pub fn new() -> BBox4 {
        BBox4 {
            min: (Float4::splat(std::f32::INFINITY),
                  Float4::splat(std::f32::INFINITY),
                  Float4::splat(std::f32::INFINITY)),
            max: (Float4::splat(std::f32::NEG_INFINITY),
                  Float4::splat(std::f32::NEG_INFINITY),
                  Float4::splat(std::f32::NEG_INFINITY)),
        }
    }

    /// Creates a BBox with min as the minimum extent and max as the maximum
    /// extent.
    pub fn from_bboxes(b1: BBox, b2: BBox, b3: BBox, b4: BBox) -> BBox4 {
        BBox4 {
            min: (Float4::new(b1.min.x(), b2.min.x(), b3.min.x(), b4.min.x()),
                  Float4::new(b1.min.y(), b2.min.y(), b3.min.y(), b4.min.y()),
                  Float4::new(b1.min.z(), b2.min.z(), b3.min.z(), b4.min.z())),
            max: (Float4::new(b1.max.x(), b2.max.x(), b3.max.x(), b4.max.x()),
                  Float4::new(b1.max.y(), b2.max.y(), b3.max.y(), b4.max.y()),
                  Float4::new(b1.max.z(), b2.max.z(), b3.max.z(), b4.max.z())),
        }
    }

    // Returns whether the given ray intersects with the bboxes.
    pub fn intersect_accel_ray(&self, ray: &AccelRay) -> Bool4 {
        // Convert ray to SIMD form
        let ray4_o =
            (Float4::splat(ray.orig.x()), Float4::splat(ray.orig.y()), Float4::splat(ray.orig.z()));
        let ray4_dinv = (Float4::splat(ray.dir_inv.x()),
                         Float4::splat(ray.dir_inv.y()),
                         Float4::splat(ray.dir_inv.z()));

        // Calculate the plane intersections
        let (xlos, xhis) = if ray.dir_inv.x() >= 0.0 {
            ((self.min.0 - ray4_o.0) * ray4_dinv.0, (self.max.0 - ray4_o.0) * ray4_dinv.0)
        } else {
            ((self.max.0 - ray4_o.0) * ray4_dinv.0, (self.min.0 - ray4_o.0) * ray4_dinv.0)
        };

        let (ylos, yhis) = if ray.dir_inv.y() >= 0.0 {
            ((self.min.1 - ray4_o.1) * ray4_dinv.1, (self.max.1 - ray4_o.1) * ray4_dinv.1)
        } else {
            ((self.max.1 - ray4_o.1) * ray4_dinv.1, (self.min.1 - ray4_o.1) * ray4_dinv.1)
        };

        let (zlos, zhis) = if ray.dir_inv.z() >= 0.0 {
            ((self.min.2 - ray4_o.2) * ray4_dinv.2, (self.max.2 - ray4_o.2) * ray4_dinv.2)
        } else {
            ((self.max.2 - ray4_o.2) * ray4_dinv.2, (self.min.2 - ray4_o.2) * ray4_dinv.2)
        };

        // Get the minimum and maximum hits
        let mins = v_max(v_max(xlos, ylos), v_max(zlos, Float4::splat(0.0)));
        let maxs = v_max(v_min(v_min(xhis, yhis), zhis),
                         Float4::splat(std::f32::NEG_INFINITY) * Float4::splat(BBOX_MAXT_ADJUST));

        // Check for hits
        let hits = mins.lt(Float4::splat(ray.max_t)) & mins.lte(maxs);

        return hits;
    }
}


/// Union of two BBoxes.
impl BitOr for BBox4 {
    type Output = BBox4;

    fn bitor(self, rhs: BBox4) -> BBox4 {
        BBox4 {
            min: (self.min.0.v_min(rhs.min.0),
                  self.min.1.v_min(rhs.min.1),
                  self.min.2.v_min(rhs.min.2)),
            max: (self.max.0.v_max(rhs.max.0),
                  self.max.1.v_max(rhs.max.1),
                  self.max.2.v_max(rhs.max.2)),
        }
    }
}

impl BitOrAssign for BBox4 {
    fn bitor_assign(&mut self, rhs: BBox4) {
        *self = *self | rhs;
    }
}

impl Lerp for BBox4 {
    fn lerp(self, other: BBox4, alpha: f32) -> BBox4 {
        BBox4 {
            min: (lerp(self.min.0, other.min.0, alpha),
                  lerp(self.min.1, other.min.1, alpha),
                  lerp(self.min.2, other.min.2, alpha)),
            max: (lerp(self.max.0, other.max.0, alpha),
                  lerp(self.max.1, other.max.1, alpha),
                  lerp(self.max.2, other.max.2, alpha)),
        }
    }
}
