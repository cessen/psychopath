mod distant_disk_light;
mod rectangle_light;
mod sphere_light;

use std::fmt::Debug;

use crate::color::SpectralSample;
use crate::math::{Matrix4x4, Normal, Point, Vector};
use crate::surface::Surface;

pub use self::distant_disk_light::DistantDiskLight;
pub use self::rectangle_light::RectangleLight;
pub use self::sphere_light::SphereLight;

/// A finite light source that can be bounded in space.
pub trait SurfaceLight: Surface {
    /// Samples the surface given a point to be illuminated.
    ///
    /// - `space`: The world-to-object space transform of the light.
    /// - `arr`: The point to be illuminated (in world space).
    /// - `u`: Random parameter U.
    /// - `v`: Random parameter V.
    /// - `wavelength`: The wavelength of light to sample at.
    /// - `time`: The time to sample at.
    ///
    /// Returns:
    /// - The light arriving at the point arr.
    /// - A tuple with the sample point on the light, the surface normal at
    ///   that point, and the point's error magnitude.  These are used
    ///   elsewhere to create a robust shadow ray.
    /// - The pdf of the sample.
    fn sample_from_point(
        &self,
        space: &Matrix4x4,
        arr: Point,
        u: f32,
        v: f32,
        wavelength: f32,
        time: f32,
    ) -> (SpectralSample, (Point, Normal, f32), f32);

    /// Returns whether the light has a delta distribution.
    ///
    /// If a light has no chance of a ray hitting it through random process
    /// then it is a delta light source.  For example, point light sources,
    /// lights that only emit in a single direction, etc.
    fn is_delta(&self) -> bool;

    /// Returns an approximation of the total energy emitted by the surface.
    ///
    /// Note: this does not need to be exact, but it does need to be non-zero
    /// for any surface that does emit light.  This is used for importance
    /// sampling.
    fn approximate_energy(&self) -> f32;
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
    fn sample_from_point(
        &self,
        u: f32,
        v: f32,
        wavelength: f32,
        time: f32,
    ) -> (SpectralSample, Vector, f32);

    /// Returns whether the light has a delta distribution.
    ///
    /// If a light has no chance of a ray hitting it through random process
    /// then it is a delta light source.  For example, point light sources,
    /// lights that only emit in a single direction, etc.
    fn is_delta(&self) -> bool;

    /// Returns an approximation of the total energy emitted by the light
    /// source.
    ///
    /// Note: this does not need to be exact, but it does need to be non-zero
    /// for any light that emits any light.  This is used for importance
    /// sampling.
    fn approximate_energy(&self) -> f32;
}
