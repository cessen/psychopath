use math::Matrix4x4;
use ray::Ray;
use surface::SurfaceIntersection;

pub struct Tracer<'a> {
    xform_stack: Vec<Matrix4x4>,
    world_rays: &'a [Ray],
    intersections: &'a mut [SurfaceIntersection],
    rays: Vec<Ray>,
}
