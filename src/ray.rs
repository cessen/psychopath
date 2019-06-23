#![allow(dead_code)]

use float4::Float4;

use crate::math::{Matrix4x4, Point, Vector};

type RayIndexType = u16;
type FlagType = u8;
const OCCLUSION_FLAG: FlagType = 1;
const DONE_FLAG: FlagType = 1 << 1;

/// This is never used directly in ray tracing--it's only used as a convenience
/// for filling the RayBatch structure.
#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub orig: Point,
    pub dir: Vector,
    pub time: f32,
    pub wavelength: f32,
    pub max_t: f32,
}

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

    pub fn push(&mut self, ray: Ray, is_occlusion: bool) {
        self.orig_world.push(ray.orig);
        self.dir_world.push(ray.dir);
        self.orig_accel.push(ray.orig); // Bogus, to place-hold.
        self.dir_inv_accel.push(ray.dir); // Bogus, to place-hold.
        self.time.push(ray.time);
        self.wavelength.push(ray.wavelength);
        if is_occlusion {
            self.max_t.push(1.0);
            self.flags.push(OCCLUSION_FLAG);
        } else {
            self.max_t.push(std::f32::INFINITY);
            self.flags.push(0);
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a != b {
            self.orig_world.swap(a, b);
            self.dir_world.swap(a, b);
            self.orig_accel.swap(a, b);
            self.dir_inv_accel.swap(a, b);
            self.max_t.swap(a, b);
            self.time.swap(a, b);
            self.wavelength.swap(a, b);
            self.flags.swap(a, b);
        }
    }

    pub fn set_from_ray(&mut self, ray: &Ray, is_shadow: bool, idx: usize) {
        self.orig_world[idx] = ray.orig;
        self.dir_world[idx] = ray.dir;
        self.orig_accel[idx] = ray.orig;
        self.dir_inv_accel[idx] = Vector {
            co: Float4::splat(1.0) / ray.dir.co,
        };
        self.max_t[idx] = ray.max_t;
        self.time[idx] = ray.time;
        self.wavelength[idx] = ray.wavelength;
        self.time[idx] = ray.time;
        self.flags[idx] = if is_shadow { OCCLUSION_FLAG } else { 0 };
    }

    pub fn truncate(&mut self, len: usize) {
        self.orig_world.truncate(len);
        self.dir_world.truncate(len);
        self.orig_accel.truncate(len);
        self.dir_inv_accel.truncate(len);
        self.max_t.truncate(len);
        self.time.truncate(len);
        self.wavelength.truncate(len);
        self.flags.truncate(len);
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

    pub fn len(&self) -> usize {
        self.orig_world.len()
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
    lanes: Vec<Lane>,
    tasks: Vec<RayTask>,
}

impl RayStack {
    pub fn new() -> RayStack {
        RayStack {
            lanes: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// Returns whether the stack is empty of tasks or not.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Makes sure there are at least `count` lanes.
    pub fn ensure_lane_count(&mut self, count: usize) {
        while self.lanes.len() < count {
            self.lanes.push(Lane {
                idxs: Vec::new(),
                end_len: 0,
            })
        }
    }

    pub fn ray_count_in_next_task(&self) -> usize {
        let task = self.tasks.last().unwrap();
        let end = self.lanes[task.lane].end_len;
        end - task.start_idx
    }

    pub fn next_task_ray_idx(&self, i: usize) -> usize {
        let task = self.tasks.last().unwrap();
        let i = i + task.start_idx;
        debug_assert!(i < self.lanes[task.lane].end_len);
        self.lanes[task.lane].idxs[i] as usize
    }

    /// Clears the lanes and tasks of the RayStack.
    ///
    /// Note: this is (importantly) different than calling clear individually
    /// on the `lanes` and `tasks` members.  Specifically, we don't want to
    /// clear `lanes` itself, as that would also free all the memory of the
    /// individual lanes.  Instead, we want to iterate over the individual
    /// lanes and clear them, but leave `lanes` itself untouched.
    pub fn clear(&mut self) {
        for lane in self.lanes.iter_mut() {
            lane.idxs.clear();
            lane.end_len = 0;
        }

        self.tasks.clear();
    }

    /// Pushes the given ray index onto the end of the specified lane.
    pub fn push_ray_index(&mut self, ray_idx: usize, lane: usize) {
        assert!(self.lanes.len() > lane);
        self.lanes[lane].idxs.push(ray_idx as RayIndexType);
    }

    /// Takes the given list of lane indices, and pushes any excess indices on
    /// the end of each into a new task, in the order provided.
    pub fn push_lanes_to_tasks(&mut self, lane_idxs: &[usize]) {
        for &l in lane_idxs {
            if self.lanes[l].end_len < self.lanes[l].idxs.len() {
                self.tasks.push(RayTask {
                    lane: l,
                    start_idx: self.lanes[l].end_len,
                });
                self.lanes[l].end_len = self.lanes[l].idxs.len();
            }
        }
    }

    /// Pops the next task off the stack, and executes the provided closure for
    /// each ray index in the task.  The return value of the closure is the list
    /// of lanes (by index) to add the given ray index back into.
    pub fn pop_do_next_task<F>(&mut self, needed_lanes: usize, mut handle_ray: F)
    where
        F: FnMut(usize) -> ([u8; 8], usize),
    {
        // Prepare lanes.
        self.ensure_lane_count(needed_lanes);

        // Pop the task and do necessary bookkeeping.
        let task = self.tasks.pop().unwrap();
        let task_range = (task.start_idx, self.lanes[task.lane].end_len);
        self.lanes[task.lane].end_len = task.start_idx;

        // Execute task.
        let mut source_lane_cap = task_range.0;
        for i in task_range.0..task_range.1 {
            let ray_idx = self.lanes[task.lane].idxs[i];
            let (add_list, list_len) = handle_ray(ray_idx as usize);
            for &l in &add_list[..list_len] {
                if l == task.lane as u8 {
                    self.lanes[l as usize].idxs[source_lane_cap] = ray_idx;
                    source_lane_cap += 1;
                } else {
                    self.lanes[l as usize].idxs.push(ray_idx);
                }
            }
        }
        self.lanes[task.lane].idxs.truncate(source_lane_cap);
    }
}

/// A lane within a RayStack.
#[derive(Debug)]
struct Lane {
    idxs: Vec<RayIndexType>,
    end_len: usize,
}

/// A task within a RayStack.
//
// Specifies the lane that the relevant ray pointers are in, and the
// starting index within that lane.  The relevant pointers are always
// `&[start_idx..]` within the given lane.
#[derive(Debug)]
struct RayTask {
    lane: usize,
    start_idx: usize,
}
