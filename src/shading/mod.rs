pub mod surface_closure;

use std::fmt::Debug;

use self::surface_closure::SurfaceClosureUnion;
use surface::SurfaceIntersectionData;

/// Trait for surface shaders.
pub trait SurfaceShader: Debug {
    /// Takes the result of a surface intersection and returns the surface
    /// closure to be evaluated at that intersection point.
    fn shade(&self, data: &SurfaceIntersectionData) -> SurfaceClosureUnion;
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
pub struct SimpleSurfaceShader {
    closure: SurfaceClosureUnion,
}

impl SimpleSurfaceShader {
    fn new(closure: SurfaceClosureUnion) -> SimpleSurfaceShader {
        SimpleSurfaceShader { closure: closure }
    }
}

impl SurfaceShader for SimpleSurfaceShader {
    fn shade(&self, data: &SurfaceIntersectionData) -> SurfaceClosureUnion {
        let _ = data; // Silence "unused" compiler warning
        self.closure
    }
}
