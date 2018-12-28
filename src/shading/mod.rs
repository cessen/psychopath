pub mod surface_closure;

use std::fmt::Debug;

use crate::{color::Color, surface::SurfaceIntersectionData};

use self::surface_closure::SurfaceClosure;

/// Trait for surface shaders.
pub trait SurfaceShader: Debug + Sync {
    /// Takes the result of a surface intersection and returns the surface
    /// closure to be evaluated at that intersection point.
    fn shade(&self, data: &SurfaceIntersectionData, time: f32, wavelength: f32) -> SurfaceClosure;
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
#[derive(Debug, Copy, Clone)]
pub enum SimpleSurfaceShader {
    Emit {
        color: Color,
    },
    Lambert {
        color: Color,
    },
    GGX {
        color: Color,
        roughness: f32,
        fresnel: f32,
    },
}

impl SurfaceShader for SimpleSurfaceShader {
    fn shade(&self, data: &SurfaceIntersectionData, time: f32, wavelength: f32) -> SurfaceClosure {
        let _ = (data, time); // Silence "unused" compiler warning

        match *self {
            SimpleSurfaceShader::Emit { color } => SurfaceClosure::Emit(color),

            SimpleSurfaceShader::Lambert { color } => SurfaceClosure::Lambert(color),

            SimpleSurfaceShader::GGX {
                color,
                roughness,
                fresnel,
            } => SurfaceClosure::GGX {
                color: color,
                roughness: roughness,
                fresnel: fresnel,
            },
        }
    }
}
