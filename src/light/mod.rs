mod sphere_light;

use std::fmt::Debug;

pub use self::sphere_light::SphereLight;

use math::{Vector, Point};
use color::SpectralSample;
use boundable::Boundable;

pub trait LightSource: Boundable + Debug + Sync {
    /// Samples the light source for a given point to be illuminated.
    ///
    ///     - arr: The point to be illuminated.
    ///     - u: Random parameter U.
    ///     - v: Random parameter V.
    ///     - wavelength: The wavelength of light to sample at.
    ///     - time: The time to sample at.
    ///
    /// Returns: The light arriving at the point arr, the vector to use for
    /// shadow testing, and the pdf of the sample.
    fn sample(&self,
              arr: Point,
              u: f32,
              v: f32,
              wavelength: f32,
              time: f32)
              -> (SpectralSample, Vector, f32);


    /// Calculates the pdf of sampling the given
    /// sample_dir/sample_u/sample_v from the given point arr.  This is used
    /// primarily to calculate probabilities for multiple importance sampling.
    ///
    /// NOTE: this function CAN assume that sample_dir, sample_u, and sample_v
    /// are a valid sample for the light source (i.e. hits/lies on the light
    /// source).  No guarantees are made about the correctness of the return
    /// value if they are not valid.
    fn sample_pdf(&self,
                  arr: Point,
                  sample_dir: Vector,
                  sample_u: f32,
                  sample_v: f32,
                  wavelength: f32,
                  time: f32)
                  -> f32;


    /// Returns the color emitted in the given direction from the
    /// given parameters on the light.
    ///
    ///     - dir: The direction of the outgoing light.
    ///     - u: Random parameter U.
    ///     - v: Random parameter V.
    ///     - wavelength: The hero wavelength of light to sample at.
    ///     - time: The time to sample at.
    fn outgoing(&self, dir: Vector, u: f32, v: f32, wavelength: f32, time: f32) -> SpectralSample;



    /// Returns whether the light has a delta distribution.
    ///
    /// If a light has no chance of a ray hitting it through random process
    /// then it is a delta light source.  For example, point light sources,
    /// lights that only emit in a single direction, etc.
    fn is_delta(&self) -> bool;
}
