use camera::Camera;
use assembly::Assembly;


#[derive(Debug)]
pub struct Scene {
    name: String,
    background_color: (f32, f32, f32),
    camera: Camera,
    root: Assembly,
}
