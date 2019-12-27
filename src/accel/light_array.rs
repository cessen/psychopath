use kioku::Arena;

use crate::{
    bbox::BBox,
    math::{Normal, Point, Vector},
    shading::surface_closure::SurfaceClosure,
};

use super::LightAccel;

#[derive(Debug, Copy, Clone)]
pub struct LightArray<'a> {
    indices: &'a [usize],
    aprx_energy: f32,
}

impl<'a> LightArray<'a> {
    #[allow(dead_code)]
    pub fn from_objects<'b, T, F>(
        arena: &'a Arena,
        objects: &mut [T],
        info_getter: F,
    ) -> LightArray<'a>
    where
        F: 'b + Fn(&T) -> (&'b [BBox], f32),
    {
        let mut indices = Vec::new();
        let mut aprx_energy = 0.0;
        for (i, thing) in objects.iter().enumerate() {
            let (_, power) = info_getter(thing);
            if power > 0.0 {
                indices.push(i);
                aprx_energy += power;
            }
        }

        LightArray {
            indices: arena.copy_slice(&indices),
            aprx_energy: aprx_energy,
        }
    }
}

impl<'a> LightAccel for LightArray<'a> {
    fn select(
        &self,
        inc: Vector,
        pos: Point,
        nor: Normal,
        nor_g: Normal,
        sc: &SurfaceClosure,
        time: f32,
        n: f32,
    ) -> Option<(usize, f32, f32)> {
        let _ = (inc, pos, nor, nor_g, sc, time); // Not using these, silence warnings

        assert!(n >= 0.0 && n <= 1.0);

        if self.indices.is_empty() {
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
