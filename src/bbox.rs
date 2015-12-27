#![allow(dead_code)]

use std;
use std::ops::BitOr;

use math::Point;
use lerp::{lerp, Lerp};

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
