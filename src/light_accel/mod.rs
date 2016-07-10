use bbox::BBox;

pub trait LightAccel {
    /// Returns (index_of_light, selection_pdf, whittled_n)
    fn select(&self, n: f32) -> Option<(usize, f32, f32)>;
}

#[derive(Debug, Clone)]
pub struct LightArray {
    indices: Vec<usize>,
}

impl LightArray {
    pub fn new<'a, T, F>(things: &mut [T], q: F) -> LightArray
        where F: 'a + Fn(&T) -> Option<(&'a [BBox], f32)>
    {
        let mut indices = Vec::new();
        for (i, thing) in things.iter().enumerate() {
            if let Some((_, power)) = q(thing) {
                if power > 0.0 {
                    indices.push(i);
                }
            }
        }

        LightArray { indices: indices }
    }
}

impl LightAccel for LightArray {
    fn select(&self, n: f32) -> Option<(usize, f32, f32)> {
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
}
