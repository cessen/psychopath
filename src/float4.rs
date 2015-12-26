#![allow(dead_code)]

use std::ops::{Index, IndexMut, Add, Sub, Mul, Div};
use std::cmp::PartialEq;

/// Essentially a tuple of four floats, which will use SIMD operations
/// where possible on a platform.
#[derive(Debug, Copy, Clone)]
pub struct Float4 {
    data: [f32; 4],
}

impl Float4 {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Float4 {
        Float4 { data: [a, b, c, d] }
    }

    pub fn h_sum(&self) -> f32 {
        self[0] + self[1] + self[2] + self[3]
    }

    pub fn h_product(&self) -> f32 {
        self[0] * self[1] * self[2] * self[3]
    }

    pub fn h_min(&self) -> f32 {
        self[0].min(self[1]).min(self[2].min(self[3]))
    }

    pub fn h_max(&self) -> f32 {
        self[0].max(self[1]).max(self[2].max(self[3]))
    }
}


impl Index<usize> for Float4 {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        &self.data[index]
    }
}

impl IndexMut<usize> for Float4 {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.data[index]
    }
}


impl PartialEq for Float4 {
    fn eq(&self, other: &Float4) -> bool {
        self.data[0] == other.data[0] && self.data[1] == other.data[1] &&
        self.data[2] == other.data[2] && self.data[3] == other.data[3]
    }
}


impl Add for Float4 {
    type Output = Float4;

    fn add(self, other: Float4) -> Float4 {
        Float4 {
            data: [self[0] + other[0], self[1] + other[1], self[2] + other[2], self[3] + other[3]],
        }
    }
}


impl Sub for Float4 {
    type Output = Float4;

    fn sub(self, other: Float4) -> Float4 {
        Float4 {
            data: [self[0] - other[0], self[1] - other[1], self[2] - other[2], self[3] - other[3]],
        }
    }
}


impl Mul for Float4 {
    type Output = Float4;

    fn mul(self, other: Float4) -> Float4 {
        Float4 {
            data: [self[0] * other[0], self[1] * other[1], self[2] * other[2], self[3] * other[3]],
        }
    }
}

impl Mul<f32> for Float4 {
    type Output = Float4;

    fn mul(self, other: f32) -> Float4 {
        Float4 { data: [self[0] * other, self[1] * other, self[2] * other, self[3] * other] }
    }
}


impl Div for Float4 {
    type Output = Float4;

    fn div(self, other: Float4) -> Float4 {
        Float4 {
            data: [self[0] / other[0], self[1] / other[1], self[2] / other[2], self[3] / other[3]],
        }
    }
}

impl Div<f32> for Float4 {
    type Output = Float4;

    fn div(self, other: f32) -> Float4 {
        Float4 { data: [self[0] / other, self[1] / other, self[2] / other, self[3] / other] }
    }
}
