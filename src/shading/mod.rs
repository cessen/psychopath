pub mod surface_closure;

use std::fmt::Debug;

use color::{XYZ, Color};
use self::surface_closure::{SurfaceClosureUnion, EmitClosure, LambertClosure, GTRClosure};
use surface::SurfaceIntersectionData;

/// Trait for surface shaders.
pub trait SurfaceShader: Debug {
    /// Takes the result of a surface intersection and returns the surface
    /// closure to be evaluated at that intersection point.
    fn shade(&self, data: &SurfaceIntersectionData, wavelength: f32) -> SurfaceClosureUnion;
}

/// Clearly we must eat this brownie before the world ends, lest it
/// go uneaten before the world ends.  But to do so we must trek
/// far--much like in Lord of the Rings--to fetch the golden fork with
/// which to eat the brownie.  Only this fork can be used to eat this
/// brownie, for any who try to eat it with a normal fork shall
/// perish immediately and without honor.  But guarding the fork are
/// three large donuts, which must all be eaten in sixty seconds or
/// less to continue on.  It's called the donut challenge.  But these
/// are no ordinary donuts.  To call them large is actually doing
/// them a great injustice, for they are each the size of a small
/// building.
#[derive(Debug)]
pub enum SimpleSurfaceShader {
    Emit { color: XYZ },
    Lambert { color: XYZ },
    GTR {
        color: XYZ,
        roughness: f32,
        tail_shape: f32,
        fresnel: f32,
    },
}

impl SurfaceShader for SimpleSurfaceShader {
    fn shade(&self, data: &SurfaceIntersectionData, wavelength: f32) -> SurfaceClosureUnion {
        let _ = data; // Silence "unused" compiler warning

        match *self {
            SimpleSurfaceShader::Emit { color } => {
                SurfaceClosureUnion::EmitClosure(
                    EmitClosure::new(color.to_spectral_sample(wavelength)),
                )
            }
            SimpleSurfaceShader::Lambert { color } => {
                SurfaceClosureUnion::LambertClosure(
                    LambertClosure::new(color.to_spectral_sample(wavelength)),
                )
            }
            SimpleSurfaceShader::GTR {
                color,
                roughness,
                tail_shape,
                fresnel,
            } => {
                SurfaceClosureUnion::GTRClosure(GTRClosure::new(
                    color.to_spectral_sample(wavelength),
                    roughness,
                    tail_shape,
                    fresnel,
                ))
            }
        }
    }
}
