#![allow(dead_code)]

use math::{Vector, Point, Matrix4x4};

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub orig: Point,
    pub dir: Vector,
}

impl Ray {
    pub fn new(orig: Point, dir: Vector) -> Ray {
        Ray {
            orig: orig,
            dir: dir,
        }
    }

    pub fn transform(&mut self, mat: &Matrix4x4) {
        self.orig = self.orig * *mat;
        self.dir = self.dir * *mat;
    }
}
