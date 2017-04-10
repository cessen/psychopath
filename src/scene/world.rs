use color::XYZ;
use light::WorldLightSource;

#[derive(Debug)]
pub struct World<'a> {
    pub background_color: XYZ,
    pub lights: &'a [&'a WorldLightSource],
}
