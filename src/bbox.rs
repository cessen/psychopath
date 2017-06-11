#![allow(dead_code)]

use std;
use std::iter::Iterator;
use std::ops::{BitOr, BitOrAssign};

use lerp::{lerp, lerp_slice, Lerp};
use math::{Point, Matrix4x4, fast_minf32};
use ray::AccelRay;


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
            max: Point::new(
                std::f32::NEG_INFINITY,
                std::f32::NEG_INFINITY,
                std::f32::NEG_INFINITY,
            ),
        }
    }

    /// Creates a BBox with min as the minimum extent and max as the maximum
    /// extent.
    pub fn from_points(min: Point, max: Point) -> BBox {
        BBox { min: min, max: max }
    }

    // Returns whether the given ray intersects with the bbox.
    pub fn intersect_accel_ray(&self, ray: &AccelRay) -> bool {
        // Calculate slab intersections
        let t1 = (self.min.co - ray.orig.co) * ray.dir_inv.co;
        let t2 = (self.max.co - ray.orig.co) * ray.dir_inv.co;

        // Find the far and near intersection
        let mut near_t = t1.v_min(t2);
        let mut far_t = t1.v_max(t2);
        near_t.set_3(0.0);
        far_t.set_3(std::f32::INFINITY);
        let hitt0 = near_t.h_max();
        let hitt1 = far_t.h_min() * BBOX_MAXT_ADJUST;

        // Did we hit?
        return hitt0 <= fast_minf32(hitt1, ray.max_t);
    }

    // Creates a new BBox transformed into a different space.
    pub fn transformed(&self, xform: Matrix4x4) -> BBox {
        // BBox corners
        let vs = [
            Point::new(self.min.x(), self.min.y(), self.min.z()),
            Point::new(self.min.x(), self.min.y(), self.max.z()),
            Point::new(self.min.x(), self.max.y(), self.min.z()),
            Point::new(self.min.x(), self.max.y(), self.max.z()),
            Point::new(self.max.x(), self.min.y(), self.min.z()),
            Point::new(self.max.x(), self.min.y(), self.max.z()),
            Point::new(self.max.x(), self.max.y(), self.min.z()),
            Point::new(self.max.x(), self.max.y(), self.max.z()),
        ];

        // Transform BBox corners and make new bbox
        let mut b = BBox::new();
        for v in vs.iter() {
            let v = *v * xform;
            b.min = v.min(b.min);
            b.max = v.max(b.max);
        }

        return b;
    }

    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        ((d.x() * d.y()) + (d.y() * d.z()) + (d.z() * d.x())) * 2.0
    }

    pub fn center(&self) -> Point {
        self.min.lerp(self.max, 0.5)
    }

    pub fn diagonal(&self) -> f32 {
        (self.max - self.min).length()
    }
}


/// Union of two BBoxes.
impl BitOr for BBox {
    type Output = BBox;

    fn bitor(self, rhs: BBox) -> BBox {
        BBox::from_points(
            Point { co: self.min.co.v_min(rhs.min.co) },
            Point { co: self.max.co.v_max(rhs.max.co) },
        )
    }
}

impl BitOrAssign for BBox {
    fn bitor_assign(&mut self, rhs: BBox) {
        *self = *self | rhs;
    }
}

/// Expand BBox by a point.
impl BitOr<Point> for BBox {
    type Output = BBox;

    fn bitor(self, rhs: Point) -> BBox {
        BBox::from_points(
            Point { co: self.min.co.v_min(rhs.co) },
            Point { co: self.max.co.v_max(rhs.co) },
        )
    }
}

impl BitOrAssign<Point> for BBox {
    fn bitor_assign(&mut self, rhs: Point) {
        *self = *self | rhs;
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


pub fn transform_bbox_slice_from(bbs_in: &[BBox], xforms: &[Matrix4x4], bbs_out: &mut Vec<BBox>) {
    bbs_out.clear();

    // Transform the bounding boxes
    if xforms.len() == 0 {
        return;
    } else if bbs_in.len() == xforms.len() {
        for (bb, xf) in Iterator::zip(bbs_in.iter(), xforms.iter()) {
            bbs_out.push(bb.transformed(xf.inverse()));
        }
    } else if bbs_in.len() > xforms.len() {
        let s = (bbs_in.len() - 1) as f32;
        for (i, bb) in bbs_in.iter().enumerate() {
            bbs_out.push(bb.transformed(lerp_slice(xforms, i as f32 / s).inverse()));
        }
    } else if bbs_in.len() < xforms.len() {
        let s = (xforms.len() - 1) as f32;
        for (i, xf) in xforms.iter().enumerate() {
            bbs_out.push(lerp_slice(bbs_in, i as f32 / s).transformed(xf.inverse()));
        }
    }
}
