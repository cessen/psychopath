use bbox::BBox;
use math::{Vector, Point, Normal};
use shading::surface_closure::SurfaceClosure;

use super::LightAccel;

#[derive(Debug, Clone)]
pub struct LightArray {
    indices: Vec<usize>,
    aprx_energy: f32,
}

impl LightArray {
    #[allow(dead_code)]
    pub fn new<'a, T, F>(things: &mut [T], q: F) -> LightArray
        where F: 'a + Fn(&T) -> Option<(&'a [BBox], f32)>
    {
        let mut indices = Vec::new();
        let mut aprx_energy = 0.0;
        for (i, thing) in things.iter().enumerate() {
            if let Some((_, power)) = q(thing) {
                if power > 0.0 {
                    indices.push(i);
                    aprx_energy += power;
                }
            }
        }

        LightArray {
            indices: indices,
            aprx_energy: aprx_energy,
        }
    }
}

impl LightAccel for LightArray {
    fn select(&self, inc: Vector, pos: Point, nor: Normal, sc: &SurfaceClosure, time: f32, n: f32) -> Option<(usize, f32, f32)> {
        let _ = (inc, pos, nor, sc, time); // Not using these, silence warnings

        assert!(n >= 0.0 && n <= 1.0);

        if self.indices.len() == 0 {
            return None;
        }

        let n2 = n * self.indices.len() as f32;
        let i = if n == 1.0 {
            *self.indices.last().unwrap()
        } else {
            self.indices[n2 as usize]
        };

        let whittled_n = n2 - i as f32;
        let pdf = 1.0 / self.indices.len() as f32;

        Some((i, pdf, whittled_n))
    }

    fn approximate_energy(&self) -> f32 {
        self.aprx_energy
    }
}
