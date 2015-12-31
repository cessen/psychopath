#![allow(dead_code)]

use std;

use float4::Float4;
use math::{Vector, Point, Matrix4x4};

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub orig: Point,
    pub dir: Vector,
    pub dir_inv: Vector,
    pub max_t: f32,
    pub time: f32,
    pub id: u32,
    pub flags: u32,
}

impl Ray {
    pub fn new(orig: Point, dir: Vector) -> Ray {
        Ray {
            orig: orig,
            dir: dir,
            dir_inv: Vector { co: Float4::new(1.0, 1.0, 1.0, 1.0) / dir.co },
            max_t: std::f32::INFINITY,
            time: 0.0,
            id: 0,
            flags: 0,
        }
    }

    pub fn transform(&mut self, mat: &Matrix4x4) {
        self.orig = self.orig * *mat;
        self.dir = self.dir * *mat;
    }
}
