#![allow(dead_code)]

use std;
use std::ops::{BitOr, BitOrAssign};

use crate::{
    bbox::BBox,
    lerp::{lerp, Lerp},
    math::{Point, Vector},
};

use float4::{Bool4, Float4};

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
            x: (
                Float4::splat(std::f32::INFINITY),
                Float4::splat(std::f32::NEG_INFINITY),
            ),
            y: (
                Float4::splat(std::f32::INFINITY),
                Float4::splat(std::f32::NEG_INFINITY),
            ),
            z: (
                Float4::splat(std::f32::INFINITY),
                Float4::splat(std::f32::NEG_INFINITY),
            ),
        }
    }

    /// Creates a BBox with min as the minimum extent and max as the maximum
    /// extent.
    pub fn from_bboxes(b1: BBox, b2: BBox, b3: BBox, b4: BBox) -> BBox4 {
        BBox4 {
            x: (
                Float4::new(b1.min.x(), b2.min.x(), b3.min.x(), b4.min.x()),
                Float4::new(b1.max.x(), b2.max.x(), b3.max.x(), b4.max.x()),
            ),
            y: (
                Float4::new(b1.min.y(), b2.min.y(), b3.min.y(), b4.min.y()),
                Float4::new(b1.max.y(), b2.max.y(), b3.max.y(), b4.max.y()),
            ),
            z: (
                Float4::new(b1.min.z(), b2.min.z(), b3.min.z(), b4.min.z()),
                Float4::new(b1.max.z(), b2.max.z(), b3.max.z(), b4.max.z()),
            ),
        }
    }

    // Returns whether the given ray intersects with the bboxes.
    pub fn intersect_ray(&self, orig: Point, dir_inv: Vector, max_t: f32) -> Bool4 {
        // Get the ray data into SIMD format.
        let ro_x = orig.co.all_0();
        let ro_y = orig.co.all_1();
        let ro_z = orig.co.all_2();
        let rdi_x = dir_inv.co.all_0();
        let rdi_y = dir_inv.co.all_1();
        let rdi_z = dir_inv.co.all_2();
        let max_t = Float4::splat(max_t);

        // Slab tests
        let t1_x = (self.x.0 - ro_x) * rdi_x;
        let t1_y = (self.y.0 - ro_y) * rdi_y;
        let t1_z = (self.z.0 - ro_z) * rdi_z;
        let t2_x = (self.x.1 - ro_x) * rdi_x;
        let t2_y = (self.y.1 - ro_y) * rdi_y;
        let t2_z = (self.z.1 - ro_z) * rdi_z;

        // Get the far and near t hits for each axis.
        let t_far_x = t1_x.v_max(t2_x);
        let t_far_y = t1_y.v_max(t2_y);
        let t_far_z = t1_z.v_max(t2_z);
        let t_near_x = t1_x.v_min(t2_x);
        let t_near_y = t1_y.v_min(t2_y);
        let t_near_z = t1_z.v_min(t2_z);

        // Calculate over-all far t hit.
        let far_t =
            (t_far_x.v_min(t_far_y.v_min(t_far_z)) * Float4::splat(BBOX_MAXT_ADJUST)).v_min(max_t);

        // Calculate over-all near t hit.
        let near_t = t_near_x
            .v_max(t_near_y)
            .v_max(t_near_z.v_max(Float4::splat(0.0)));

        // Hit results
        near_t.lt(far_t)
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
            x: (
                lerp(self.x.0, other.x.0, alpha),
                lerp(self.x.1, other.x.1, alpha),
            ),
            y: (
                lerp(self.y.0, other.y.0, alpha),
                lerp(self.y.1, other.y.1, alpha),
            ),
            z: (
                lerp(self.z.0, other.z.0, alpha),
                lerp(self.z.1, other.z.1, alpha),
            ),
        }
    }
}
