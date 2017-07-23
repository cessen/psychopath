use accel::LightAccel;
use algorithm::weighted_choice;
use camera::Camera;
use color::SpectralSample;
use math::Vector;
use surface::SurfaceIntersection;
use transform_stack::TransformStack;

use super::Assembly;
use super::World;


#[derive(Debug)]
pub struct Scene<'a> {
    pub name: Option<String>,
    pub camera: Camera<'a>,
    pub world: World<'a>,
    pub root: Assembly<'a>,
}

impl<'a> Scene<'a> {
    pub fn sample_lights(
        &self,
        xform_stack: &mut TransformStack,
        n: f32,
        uvw: (f32, f32, f32),
        wavelength: f32,
        time: f32,
        intr: &SurfaceIntersection,
    ) -> Option<(SpectralSample, Vector, f32, f32, bool)> {
        // TODO: this just selects between world lights and local lights
        // with a 50/50 chance.  We should do something more sophisticated
        // than this, accounting for the estimated impact of the lights
        // on the point being lit.

        // Calculate relative probabilities of traversing into world lights
        // or local lights.
        let wl_energy = if self.world.lights.iter().fold(0.0, |energy, light| {
            energy + light.approximate_energy()
        }) <= 0.0
        {
            0.0
        } else {
            1.0
        };
        let ll_energy = if self.root.light_accel.approximate_energy() <= 0.0 {
            0.0
        } else {
            1.0
        };
        let tot_energy = wl_energy + ll_energy;

        // Decide either world or local lights, and select and sample a light.
        if tot_energy <= 0.0 {
            return None;
        } else {
            let wl_prob = wl_energy / tot_energy;

            if n < wl_prob {
                // World lights
                let n = n / wl_prob;
                let (i, p) = weighted_choice(self.world.lights, n, |l| l.approximate_energy());
                let (ss, sv, pdf) = self.world.lights[i].sample(uvw.0, uvw.1, wavelength, time);
                return Some((ss, sv, pdf, p * wl_prob, true));
            } else {
                // Local lights
                let n = (n - wl_prob) / (1.0 - wl_prob);

                if let Some((ss, sv, pdf, spdf)) =
                    self.root.sample_lights(
                        xform_stack,
                        n,
                        uvw,
                        wavelength,
                        time,
                        intr,
                    )
                {
                    return Some((ss, sv, pdf, spdf * (1.0 - wl_prob), false));
                } else {
                    return None;
                }
            }
        }
    }
}
