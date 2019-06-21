#![allow(dead_code)]

use float4::Float4;

use crate::math::{Matrix4x4, Point, Vector};

type FlagType = u8;
const OCCLUSION_FLAG: FlagType = 1;
const DONE_FLAG: FlagType = 1 << 1;

/// A batch of rays, stored in SoA layout.
#[derive(Debug)]
pub struct RayBatch {
    pub orig_world: Vec<Point>,
    pub dir_world: Vec<Vector>,
    pub orig_accel: Vec<Point>,
    pub dir_inv_accel: Vec<Vector>,
    pub max_t: Vec<f32>,
    pub time: Vec<f32>,
    pub wavelength: Vec<f32>,
    pub flags: Vec<FlagType>,
}

impl RayBatch {
    /// Creates a new empty ray batch.
    pub fn new() -> RayBatch {
        RayBatch {
            orig_world: Vec::new(),
            dir_world: Vec::new(),
            orig_accel: Vec::new(),
            dir_inv_accel: Vec::new(),
            max_t: Vec::new(),
            time: Vec::new(),
            wavelength: Vec::new(),
            flags: Vec::new(),
        }
    }

    /// Creates a new empty ray batch, with pre-allocated capacity for
    /// `n` rays.
    pub fn with_capacity(n: usize) -> RayBatch {
        RayBatch {
            orig_world: Vec::with_capacity(n),
            dir_world: Vec::with_capacity(n),
            orig_accel: Vec::with_capacity(n),
            dir_inv_accel: Vec::with_capacity(n),
            max_t: Vec::with_capacity(n),
            time: Vec::with_capacity(n),
            wavelength: Vec::with_capacity(n),
            flags: Vec::with_capacity(n),
        }
    }

    /// Clear all rays, settings the size of the batch back to zero.
    ///
    /// Capacity is maintained.
    pub fn clear(&mut self) {
        self.orig_world.clear();
        self.dir_world.clear();
        self.orig_accel.clear();
        self.dir_inv_accel.clear();
        self.max_t.clear();
        self.time.clear();
        self.wavelength.clear();
        self.flags.clear();
    }

    /// Returns whether the given ray (at index `idx`) is an occlusion ray.
    pub fn is_occlusion(&self, idx: usize) -> bool {
        (self.flags[idx] & OCCLUSION_FLAG) != 0
    }

    /// Returns whether the given ray (at index `idx`) has finished traversal.
    pub fn is_done(&self, idx: usize) -> bool {
        (self.flags[idx] & DONE_FLAG) != 0
    }

    /// Marks the given ray (at index `idx`) as an occlusion ray.
    pub fn mark_occlusion(&mut self, idx: usize) {
        self.flags[idx] |= OCCLUSION_FLAG
    }

    /// Marks the given ray (at index `idx`) as having finished traversal.
    pub fn mark_done(&mut self, idx: usize) {
        self.flags[idx] |= DONE_FLAG
    }

    /// Updates the accel data of the given ray (at index `idx`) with the
    /// given world-to-local-space transform matrix.
    ///
    /// This should be called when entering (and exiting) traversal of a
    /// new transform space.
    pub fn update_accel(&mut self, idx: usize, xform: &Matrix4x4) {
        self.orig_accel[idx] = self.orig_world[idx] * *xform;
        self.dir_inv_accel[idx] = Vector {
            co: Float4::splat(1.0) / (self.dir_world[idx] * *xform).co,
        };
    }
}

/// A structure used for tracking traversal of a ray batch through a scene.
#[derive(Debug)]
pub struct RayStack {
    lanes: Vec<Vec<u16>>,
    tasks: Vec<RayTask>,
}

/// A task within a RayStack.
#[derive(Debug)]
pub enum RayTask {
    // A barrier represents a division when traversing into a new system.
    // For example, when traversing from the top-level BVH into an object's
    // local BVH.  It helps with keeping track of where we're at and aids in
    // debugging.
    Barrier,

    // A task for handling a set of rays.
    //
    // Specifies the lane that the relevant ray pointers are in, and the
    // starting index within that lane.  The relevant pointers are always
    // `&[start_idx..]` within the given lane.
    Rays { lane: usize, start_idx: usize },
}

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub orig: Point,
    pub dir: Vector,
    pub max_t: f32,
    pub time: f32,
    pub wavelength: f32,
    pub flags: FlagType,
}

impl Ray {
    pub fn new(orig: Point, dir: Vector, time: f32, wavelength: f32, is_occ: bool) -> Ray {
        if !is_occ {
            Ray {
                orig: orig,
                dir: dir,
                max_t: std::f32::INFINITY,
                time: time,
                wavelength: wavelength,
                flags: 0,
            }
        } else {
            Ray {
                orig: orig,
                dir: dir,
                max_t: 1.0,
                time: time,
                wavelength: wavelength,
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
    pub flags: FlagType,
    pub id: u32,
}

impl AccelRay {
    pub fn new(ray: &Ray, id: u32) -> AccelRay {
        AccelRay {
            orig: ray.orig,
            dir_inv: Vector {
                co: Float4::splat(1.0) / ray.dir.co,
            },
            max_t: ray.max_t,
            time: ray.time,
            flags: ray.flags,
            id: id,
        }
    }

    pub fn update_from_world_ray(&mut self, wr: &Ray) {
        self.orig = wr.orig;
        self.dir_inv = Vector {
            co: Float4::splat(1.0) / wr.dir.co,
        };
    }

    pub fn update_from_xformed_world_ray(&mut self, wr: &Ray, mat: &Matrix4x4) {
        self.orig = wr.orig * *mat;
        self.dir_inv = Vector {
            co: Float4::splat(1.0) / (wr.dir * *mat).co,
        };
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
