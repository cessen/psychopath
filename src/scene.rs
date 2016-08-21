use assembly::Assembly;
use camera::Camera;
use color::XYZ;


#[derive(Debug)]
pub struct Scene {
    pub name: Option<String>,
    pub background_color: XYZ,
    pub camera: Camera,
    pub root: Assembly,
}
