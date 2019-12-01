mod assembly;
mod world;

use std::collections::HashMap;

use crate::{camera::Camera, shading::SurfaceShader};

pub use self::{
    assembly::{Assembly, Object, ObjectData},
    world::World,
};

#[derive(Debug)]
pub struct Scene<'a> {
    pub camera: Camera<'a>,
    pub world: World<'a>,
    pub shaders: HashMap<String, Box<dyn SurfaceShader>>, // Name, Shader
    pub root_assembly: Assembly<'a>,
}
