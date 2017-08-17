mod distant_disk_light;
mod rectangle_light;
mod sphere_light;

use std::fmt::Debug;

use boundable::Boundable;
use color::SpectralSample;
use math::{Vector, Point, Matrix4x4};
use ray::{Ray, AccelRay};
use surface::SurfaceIntersection;

pub use self::distant_disk_light::DistantDiskLight;
pub use self::rectangle_light::RectangleLight;
pub use self::sphere_light::SphereLight;


/// A finite light source that can be bounded in space.
pub trait LightSource: Boundable + Debug + Sync {
    /// Samples the light source for a given point to be illuminated.
    ///
    ///     - space: The world-to-object space transform of the light.
    ///     - arr: The point to be illuminated (in world space).
    ///     - u: Random parameter U.
    ///     - v: Random parameter V.
    ///     - wavelength: The wavelength of light to sample at.
    ///     - time: The time to sample at.
    ///
    /// Returns: The light arriving at the point arr, the vector to use for
    /// shadow testing, and the pdf of the sample.
    fn sample(
        &self,
        space: &Matrix4x4,
        arr: Point,
        u: f32,
        v: f32,
        wavelength: f32,
        time: f32,
    ) -> (SpectralSample, Vector, f32);


    /// Calculates the pdf of sampling the given
    /// `sample_dir`/`sample_u`/`sample_v` from the given point `arr`.  This is used
    /// primarily to calculate probabilities for multiple importance sampling.
    ///
    /// NOTE: this function CAN assume that sample_dir, sample_u, and sample_v
    /// are a valid sample for the light source (i.e. hits/lies on the light
    /// source).  No guarantees are made about the correctness of the return
    /// value if they are not valid.
    ///
    /// TODO: this probably shouldn't be part of the public interface.  In the
    /// rest of the renderer, the PDF is always calculated by the `sample` and
    /// and `intersect_rays` methods.
    fn sample_pdf(
        &self,
        space: &Matrix4x4,
        arr: Point,
        sample_dir: Vector,
        sample_u: f32,
        sample_v: f32,
        wavelength: f32,
        time: f32,
    ) -> f32;


    /// Returns the color emitted in the given direction from the
    /// given parameters on the light.
    ///
    ///     - dir: The direction of the outgoing light.
    ///     - u: Random parameter U.
    ///     - v: Random parameter V.
    ///     - wavelength: The hero wavelength of light to sample at.
    ///     - time: The time to sample at.
    ///
    /// TODO: this probably shouldn't be part of the public interface.  In the
    /// rest of the renderer, this is handled by the `sample` and
    /// `intersect_rays` methods.
    fn outgoing(
        &self,
        space: &Matrix4x4,
        dir: Vector,
        u: f32,
        v: f32,
        wavelength: f32,
        time: f32,
    ) -> SpectralSample;


    /// Returns whether the light has a delta distribution.
    ///
    /// If a light has no chance of a ray hitting it through random process
    /// then it is a delta light source.  For example, point light sources,
    /// lights that only emit in a single direction, etc.
    fn is_delta(&self) -> bool;


    /// Returns an approximation of the total energy emitted by the light
    /// source.  Note that this does not need to be exact: it is used for
    /// importance sampling.
    fn approximate_energy(&self) -> f32;

    fn intersect_rays(
        &self,
        accel_rays: &mut [AccelRay],
        wrays: &[Ray],
        isects: &mut [SurfaceIntersection],
        space: &[Matrix4x4],
    );
}


/// An infinite light source that cannot be bounded in space.  E.g.
/// a sun light source.
pub trait WorldLightSource: Debug + Sync {
    /// Samples the light source for a given point to be illuminated.
    ///
    ///     - u: Random parameter U.
    ///     - v: Random parameter V.
    ///     - wavelength: The wavelength of light to sample at.
    ///     - time: The time to sample at.
    ///
    /// Returns: The light arriving from the shadow-testing direction, the
    /// vector to use for shadow testing, and the pdf of the sample.
    fn sample(&self, u: f32, v: f32, wavelength: f32, time: f32) -> (SpectralSample, Vector, f32);


    /// Calculates the pdf of sampling the given sample_dir.  This is used
    /// primarily to calculate probabilities for multiple importance sampling.
    ///
    /// NOTE: this function CAN assume that sample_dir is a valid sample for
    /// the light source (i.e. hits/lies on the light source).  No guarantees
    /// are made about the correctness of the return value if it isn't valid.
    fn sample_pdf(&self, sample_dir: Vector, wavelength: f32, time: f32) -> f32;


    /// Returns the color emitted in the given direction from the
    /// given parameters on the light.
    ///
    ///     - dir: The direction of the outgoing light.
    ///     - wavelength: The hero wavelength of light to sample at.
    ///     - time: The time to sample at.
    fn outgoing(&self, dir: Vector, wavelength: f32, time: f32) -> SpectralSample;


    /// Returns whether the light has a delta distribution.
    ///
    /// If a light has no chance of a ray hitting it through random process
    /// then it is a delta light source.  For example, point light sources,
    /// lights that only emit in a single direction, etc.
    fn is_delta(&self) -> bool;


    /// Returns an approximation of the total energy emitted by the light
    /// source.  Note that this does not need to be exact: it is used for
    /// importance sampling.
    fn approximate_energy(&self) -> f32;
}
