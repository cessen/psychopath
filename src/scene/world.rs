use crate::{color::Color, light::WorldLightSource};

#[derive(Debug)]
pub struct World<'a> {
    pub background_color: Color,
    pub lights: &'a [&'a WorldLightSource],
}
