use camera::Camera;
use assembly::Assembly;


#[derive(Debug)]
pub struct Scene {
    pub name: Option<String>,
    pub background_color: (f32, f32, f32),
    pub camera: Camera,
    pub root: Assembly,
}
