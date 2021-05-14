#![allow(dead_code)]

use std;
use std::ops::{BitOr, BitOrAssign};

use crate::{
    bbox::BBox,
    lerp::{lerp, Lerp},
    math::{Point, Vector},
};

use glam::{BVec4A, Vec4};

const BBOX_MAXT_ADJUST: f32 = 1.000_000_24;

/// A SIMD set of 4 3D axis-aligned bounding boxes.
#[derive(Debug, Copy, Clone)]
pub struct BBox4 {
    pub x: (Vec4, Vec4), // (min, max)
    pub y: (Vec4, Vec4), // (min, max)
    pub z: (Vec4, Vec4), // (min, max)
}

impl BBox4 {
    /// Creates a degenerate BBox with +infinity min and -infinity max.
    pub fn new() -> BBox4 {
        BBox4 {
            x: (
                Vec4::splat(std::f32::INFINITY),
                Vec4::splat(std::f32::NEG_INFINITY),
            ),
            y: (
                Vec4::splat(std::f32::INFINITY),
                Vec4::splat(std::f32::NEG_INFINITY),
            ),
            z: (
                Vec4::splat(std::f32::INFINITY),
                Vec4::splat(std::f32::NEG_INFINITY),
            ),
        }
    }

    /// Creates a BBox with min as the minimum extent and max as the maximum
    /// extent.
    pub fn from_bboxes(b1: BBox, b2: BBox, b3: BBox, b4: BBox) -> BBox4 {
        BBox4 {
            x: (
                Vec4::new(b1.min.x(), b2.min.x(), b3.min.x(), b4.min.x()),
                Vec4::new(b1.max.x(), b2.max.x(), b3.max.x(), b4.max.x()),
            ),
            y: (
                Vec4::new(b1.min.y(), b2.min.y(), b3.min.y(), b4.min.y()),
                Vec4::new(b1.max.y(), b2.max.y(), b3.max.y(), b4.max.y()),
            ),
            z: (
                Vec4::new(b1.min.z(), b2.min.z(), b3.min.z(), b4.min.z()),
                Vec4::new(b1.max.z(), b2.max.z(), b3.max.z(), b4.max.z()),
            ),
        }
    }

    // Returns whether the given ray intersects with the bboxes.
    pub fn intersect_ray(&self, orig: Point, dir_inv: Vector, max_t: f32) -> BVec4A {
        // Get the ray data into SIMD format.
        let ro_x = Vec4::splat(orig.co[0]);
        let ro_y = Vec4::splat(orig.co[1]);
        let ro_z = Vec4::splat(orig.co[2]);
        let rdi_x = Vec4::splat(dir_inv.co[0]);
        let rdi_y = Vec4::splat(dir_inv.co[1]);
        let rdi_z = Vec4::splat(dir_inv.co[2]);
        let max_t = Vec4::splat(max_t);

        // Slab tests
        let t1_x = (self.x.0 - ro_x) * rdi_x;
        let t1_y = (self.y.0 - ro_y) * rdi_y;
        let t1_z = (self.z.0 - ro_z) * rdi_z;
        let t2_x = (self.x.1 - ro_x) * rdi_x;
        let t2_y = (self.y.1 - ro_y) * rdi_y;
        let t2_z = (self.z.1 - ro_z) * rdi_z;

        // Get the far and near t hits for each axis.
        let t_far_x = t1_x.max(t2_x);
        let t_far_y = t1_y.max(t2_y);
        let t_far_z = t1_z.max(t2_z);
        let t_near_x = t1_x.min(t2_x);
        let t_near_y = t1_y.min(t2_y);
        let t_near_z = t1_z.min(t2_z);

        // Calculate over-all far t hit.
        let far_t = (t_far_x.min(t_far_y.min(t_far_z)) * Vec4::splat(BBOX_MAXT_ADJUST)).min(max_t);

        // Calculate over-all near t hit.
        let near_t = t_near_x.max(t_near_y).max(t_near_z.max(Vec4::splat(0.0)));

        // Hit results
        near_t.cmplt(far_t)
    }
}

/// Union of two BBoxes.
impl BitOr for BBox4 {
    type Output = BBox4;

    fn bitor(self, rhs: BBox4) -> BBox4 {
        BBox4 {
            x: (self.x.0.min(rhs.x.0), self.x.1.max(rhs.x.1)),
            y: (self.y.0.min(rhs.y.0), self.y.1.max(rhs.y.1)),
            z: (self.z.0.min(rhs.z.0), self.z.1.max(rhs.z.1)),
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
