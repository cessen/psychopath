#![allow(dead_code)]

use std::result::Result;

use nom;
use nom::IResult;

use super::DataTree;
use super::basics::{ws_u32, ws_f32};

use math::Matrix4x4;
use camera::Camera;
use renderer::Renderer;
use assembly::Assembly;

pub fn parse_assembly(tree: &DataTree) -> Result<Assembly, ()> {
    unimplemented!()
}
