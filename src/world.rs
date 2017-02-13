use color::XYZ;
use light::WorldLightSource;

#[derive(Debug)]
pub struct World {
    pub background_color: XYZ,
    pub lights: Vec<Box<WorldLightSource>>,
}
