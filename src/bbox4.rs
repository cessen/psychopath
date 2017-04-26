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
    pub x: (Float4, Float4), // (min, max)
    pub y: (Float4, Float4), // (min, max)
    pub z: (Float4, Float4), // (min, max)
}

impl BBox4 {
    /// Creates a degenerate BBox with +infinity min and -infinity max.
    pub fn new() -> BBox4 {
        BBox4 {
            x: (Float4::splat(std::f32::INFINITY), Float4::splat(std::f32::NEG_INFINITY)),
            y: (Float4::splat(std::f32::INFINITY), Float4::splat(std::f32::NEG_INFINITY)),
            z: (Float4::splat(std::f32::INFINITY), Float4::splat(std::f32::NEG_INFINITY)),
        }
    }

    /// Creates a BBox with min as the minimum extent and max as the maximum
    /// extent.
    pub fn from_bboxes(b1: BBox, b2: BBox, b3: BBox, b4: BBox) -> BBox4 {
        BBox4 {
            x: (Float4::new(b1.min.x(), b2.min.x(), b3.min.x(), b4.min.x()),
                Float4::new(b1.max.x(), b2.max.x(), b3.max.x(), b4.max.x())),
            y: (Float4::new(b1.min.y(), b2.min.y(), b3.min.y(), b4.min.y()),
                Float4::new(b1.max.y(), b2.max.y(), b3.max.y(), b4.max.y())),
            z: (Float4::new(b1.min.z(), b2.min.z(), b3.min.z(), b4.min.z()),
                Float4::new(b1.max.z(), b2.max.z(), b3.max.z(), b4.max.z())),
        }
    }

    // Returns whether the given ray intersects with the bboxes.
    pub fn intersect_accel_ray(&self, ray: &AccelRay) -> Bool4 {
        // Precalculate ray direction sign booleans.
        // Doing it up here slightly speeds things up lower down.
        let ray_pos = (ray.dir_inv.x() >= 0.0, ray.dir_inv.y() >= 0.0, ray.dir_inv.z() >= 0.0);

        // Convert ray to SIMD form
        let ray4_o =
            (Float4::splat(ray.orig.x()), Float4::splat(ray.orig.y()), Float4::splat(ray.orig.z()));
        let ray4_dinv = (Float4::splat(ray.dir_inv.x()),
                         Float4::splat(ray.dir_inv.y()),
                         Float4::splat(ray.dir_inv.z()));

        // Calculate the plane intersections
        let (xlos, xhis) = if ray_pos.0 {
            ((self.x.0 - ray4_o.0) * ray4_dinv.0, (self.x.1 - ray4_o.0) * ray4_dinv.0)
        } else {
            ((self.x.1 - ray4_o.0) * ray4_dinv.0, (self.x.0 - ray4_o.0) * ray4_dinv.0)
        };
        let (ylos, yhis) = if ray_pos.1 {
            ((self.y.0 - ray4_o.1) * ray4_dinv.1, (self.y.1 - ray4_o.1) * ray4_dinv.1)
        } else {
            ((self.y.1 - ray4_o.1) * ray4_dinv.1, (self.y.0 - ray4_o.1) * ray4_dinv.1)
        };
        let (zlos, zhis) = if ray_pos.2 {
            ((self.z.0 - ray4_o.2) * ray4_dinv.2, (self.z.1 - ray4_o.2) * ray4_dinv.2)
        } else {
            ((self.z.1 - ray4_o.2) * ray4_dinv.2, (self.z.0 - ray4_o.2) * ray4_dinv.2)
        };

        // Get the minimum and maximum hits
        let mins = v_max(v_max(xlos, ylos), v_max(zlos, Float4::splat(0.0)));
        let maxs = v_max(v_min(v_min(xhis, yhis), zhis),
                         Float4::splat(std::f32::NEG_INFINITY)) *
                   Float4::splat(BBOX_MAXT_ADJUST);

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
            x: (self.x.0.v_min(rhs.x.0), self.x.1.v_max(rhs.x.1)),
            y: (self.y.0.v_min(rhs.y.0), self.y.1.v_max(rhs.y.1)),
            z: (self.z.0.v_min(rhs.z.0), self.z.1.v_max(rhs.z.1)),
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
            x: (lerp(self.x.0, other.x.0, alpha), lerp(self.x.1, other.x.1, alpha)),
            y: (lerp(self.y.0, other.y.0, alpha), lerp(self.y.1, other.y.1, alpha)),
            z: (lerp(self.z.0, other.z.0, alpha), lerp(self.z.1, other.z.1, alpha)),
        }
    }
}
