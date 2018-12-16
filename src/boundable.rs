#![allow(dead_code)]

use crate::bbox::BBox;

pub trait Boundable {
    fn bounds(&self) -> &[BBox];
}
