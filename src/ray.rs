#![allow(dead_code)]

use std;

use math::{Vector, Point, Matrix4x4};

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub orig: Point,
    pub dir: Vector,
    pub max_t: f32,
    pub time: f32,
}

impl Ray {
    pub fn new(orig: Point, dir: Vector) -> Ray {
        Ray {
            orig: orig,
            dir: dir,
            max_t: std::f32::INFINITY,
            time: 0.0,
        }
    }

    pub fn transform(&mut self, mat: &Matrix4x4) {
        self.orig = self.orig * *mat;
        self.dir = self.dir * *mat;
    }
}