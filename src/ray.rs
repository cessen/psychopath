#![allow(dead_code)]

use std;

use bitstack::BitStack128;
use float4::Float4;
use math::{Vector, Point, Matrix4x4};


const OCCLUSION_FLAG: u32 = 1;
const DONE_FLAG: u32 = 1 << 1;

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub orig: Point,
    pub dir: Vector,
    pub max_t: f32,
    pub time: f32,
    pub flags: u32,
}

impl Ray {
    pub fn new(orig: Point, dir: Vector, time: f32, is_occ: bool) -> Ray {
        if !is_occ {
            Ray {
                orig: orig,
                dir: dir,
                max_t: std::f32::INFINITY,
                time: time,
                flags: 0,
            }
        } else {
            Ray {
                orig: orig,
                dir: dir,
                max_t: 1.0,
                time: time,
                flags: OCCLUSION_FLAG,
            }
        }
    }

    pub fn transform(&mut self, mat: &Matrix4x4) {
        self.orig = self.orig * *mat;
        self.dir = self.dir * *mat;
    }

    pub fn is_occlusion(&self) -> bool {
        (self.flags & OCCLUSION_FLAG) != 0
    }
}


#[derive(Debug, Copy, Clone)]
pub struct AccelRay {
    pub orig: Point,
    pub dir_inv: Vector,
    pub max_t: f32,
    pub time: f32,
    pub flags: u32,
    pub id: u32,
    pub trav_stack: BitStack128,
}

impl AccelRay {
    pub fn new(ray: &Ray, id: u32) -> AccelRay {
        AccelRay {
            orig: ray.orig,
            dir_inv: Vector { co: Float4::new(1.0, 1.0, 1.0, 1.0) / ray.dir.co },
            max_t: ray.max_t,
            time: ray.time,
            flags: ray.flags,
            id: id,
            trav_stack: BitStack128::new_with_1(),
        }
    }

    pub fn update_from_world_ray(&mut self, wr: &Ray) {
        self.orig = wr.orig;
        self.dir_inv = Vector { co: Float4::new(1.0, 1.0, 1.0, 1.0) / wr.dir.co };
    }

    pub fn update_from_xformed_world_ray(&mut self, wr: &Ray, mat: &Matrix4x4) {
        self.orig = wr.orig * *mat;
        self.dir_inv = Vector { co: Float4::new(1.0, 1.0, 1.0, 1.0) / (wr.dir * *mat).co };
    }

    pub fn is_occlusion(&self) -> bool {
        (self.flags & OCCLUSION_FLAG) != 0
    }

    pub fn is_done(&self) -> bool {
        (self.flags & DONE_FLAG) != 0
    }

    pub fn mark_done(&mut self) {
        self.flags |= DONE_FLAG;
    }
}
