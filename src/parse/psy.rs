#![allow(dead_code)]

use std::result::Result;

use nom;
use nom::IResult;

use mem_arena::MemArena;

use camera::Camera;
use color::{XYZ, rec709e_to_xyz};
use light::WorldLightSource;
use math::Matrix4x4;
use renderer::Renderer;
use scene::Scene;
use scene::World;

use super::basics::{ws_u32, ws_f32};
use super::DataTree;
use super::psy_assembly::parse_assembly;
use super::psy_light::parse_distant_disk_light;


#[derive(Copy, Clone, Debug)]
pub enum PsyParseError {
    UnknownError,
    SectionWrongCount(&'static str, usize),
}


/// Takes in a DataTree representing a Scene node and returns
pub fn parse_scene<'a>(arena: &'a MemArena,
                       tree: &'a DataTree)
                       -> Result<Renderer<'a>, PsyParseError> {
    // Verify we have the right number of each section
    if tree.iter_children_with_type("Output").count() != 1 {
        let count = tree.iter_children_with_type("Output").count();
        return Err(PsyParseError::SectionWrongCount("Output", count));
    }
    if tree.iter_children_with_type("RenderSettings").count() != 1 {
        let count = tree.iter_children_with_type("RenderSettings").count();
        return Err(PsyParseError::SectionWrongCount("RenderSettings", count));
    }
    if tree.iter_children_with_type("Camera").count() != 1 {
        let count = tree.iter_children_with_type("Camera").count();
        return Err(PsyParseError::SectionWrongCount("Camera", count));
    }
    if tree.iter_children_with_type("World").count() != 1 {
        let count = tree.iter_children_with_type("World").count();
        return Err(PsyParseError::SectionWrongCount("World", count));
    }
    if tree.iter_children_with_type("Assembly").count() != 1 {
        let count = tree.iter_children_with_type("Assembly").count();
        return Err(PsyParseError::SectionWrongCount("Root Assembly", count));
    }

    // Parse output info
    let output_info = parse_output_info(tree.iter_children_with_type("Output")
        .nth(0)
        .unwrap())?;

    // Parse render settings
    let render_settings = parse_render_settings(tree.iter_children_with_type("RenderSettings")
        .nth(0)
        .unwrap())?;

    // Parse camera
    let camera = parse_camera(arena,
                              tree.iter_children_with_type("Camera").nth(0).unwrap())?;

    // Parse world
    let world = parse_world(arena, tree.iter_children_with_type("World").nth(0).unwrap())?;

    // Parse root scene assembly
    let assembly = parse_assembly(tree.iter_children_with_type("Assembly").nth(0).unwrap())?;

    // Put scene together
    let scene_name = if let &DataTree::Internal { ident, .. } = tree {
        if let Some(name) = ident {
            Some(name.to_string())
        } else {
            None
        }
    } else {
        None
    };
    let scene = Scene {
        name: scene_name,
        camera: camera,
        world: world,
        root: assembly,
    };

    // Put renderer together
    let renderer = Renderer {
        output_file: output_info.clone(),
        resolution: ((render_settings.0).0 as usize, (render_settings.0).1 as usize),
        spp: render_settings.1 as usize,
        seed: render_settings.2,
        scene: scene,
    };

    return Ok(renderer);
}




fn parse_output_info(tree: &DataTree) -> Result<String, PsyParseError> {
    if let &DataTree::Internal { ref children, .. } = tree {
        let mut found_path = false;
        let mut path = String::new();

        for child in children {
            match child {
                &DataTree::Leaf { type_name, contents } if type_name == "Path" => {
                    // Trim and validate
                    let tc = contents.trim();
                    if tc.chars().count() < 2 {
                        // TODO: proper error
                        panic!();
                    }
                    if tc.chars().nth(0).unwrap() != '"' || tc.chars().last().unwrap() != '"' {
                        // TODO: proper error
                        panic!();
                    }
                    let len = tc.len();
                    let tc = &tc[1..len - 1];

                    // Parse
                    // TODO: proper string escaping
                    found_path = true;
                    path = tc.to_string();
                }

                _ => {}
            }
        }

        if found_path {
            return Ok((path));
        } else {
            return Err(PsyParseError::UnknownError);
        }
    } else {
        return Err(PsyParseError::UnknownError);
    };
}




fn parse_render_settings(tree: &DataTree) -> Result<((u32, u32), u32, u32), PsyParseError> {
    if let &DataTree::Internal { ref children, .. } = tree {
        let mut found_res = false;
        let mut found_spp = false;
        let mut res = (0, 0);
        let mut spp = 0;
        let mut seed = 0;

        for child in children {
            match child {
                // Resolution
                &DataTree::Leaf { type_name, contents } if type_name == "Resolution" => {
                    if let IResult::Done(_, (w, h)) = closure!(terminated!(tuple!(ws_u32, ws_u32),
                                                nom::eof))(contents.as_bytes()) {
                        found_res = true;
                        res = (w, h);
                    } else {
                        // Found Resolution, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                // SamplesPerPixel
                &DataTree::Leaf { type_name, contents } if type_name == "SamplesPerPixel" => {
                    if let IResult::Done(_, n) = ws_u32(contents.as_bytes()) {
                        found_spp = true;
                        spp = n;
                    } else {
                        // Found SamplesPerPixel, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                // Seed
                &DataTree::Leaf { type_name, contents } if type_name == "Seed" => {
                    if let IResult::Done(_, n) = ws_u32(contents.as_bytes()) {
                        seed = n;
                    } else {
                        // Found Seed, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                _ => {}
            }
        }

        if found_res && found_spp {
            return Ok((res, spp, seed));
        } else {
            return Err(PsyParseError::UnknownError);
        }
    } else {
        return Err(PsyParseError::UnknownError);
    };
}




fn parse_camera<'a>(arena: &'a MemArena, tree: &'a DataTree) -> Result<Camera<'a>, PsyParseError> {
    if let &DataTree::Internal { ref children, .. } = tree {
        let mut mats = Vec::new();
        let mut fovs = Vec::new();
        let mut focus_distances = Vec::new();
        let mut aperture_radii = Vec::new();

        // Parse
        for child in children.iter() {
            match child {
                // Fov
                &DataTree::Leaf { type_name, contents } if type_name == "Fov" => {
                    if let IResult::Done(_, fov) = ws_f32(contents.as_bytes()) {
                        fovs.push(fov * (3.1415926536 / 180.0));
                    } else {
                        // Found Fov, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                // FocalDistance
                &DataTree::Leaf { type_name, contents } if type_name == "FocalDistance" => {
                    if let IResult::Done(_, fd) = ws_f32(contents.as_bytes()) {
                        focus_distances.push(fd);
                    } else {
                        // Found FocalDistance, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                // ApertureRadius
                &DataTree::Leaf { type_name, contents } if type_name == "ApertureRadius" => {
                    if let IResult::Done(_, ar) = ws_f32(contents.as_bytes()) {
                        aperture_radii.push(ar);
                    } else {
                        // Found ApertureRadius, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                // Transform
                &DataTree::Leaf { type_name, contents } if type_name == "Transform" => {
                    if let Ok(mat) = parse_matrix(contents) {
                        mats.push(mat);
                    } else {
                        // Found Transform, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                _ => {}
            }
        }

        return Ok(Camera::new(arena, mats, fovs, aperture_radii, focus_distances));
    } else {
        return Err(PsyParseError::UnknownError);
    }
}




fn parse_world<'a>(arena: &'a MemArena, tree: &'a DataTree) -> Result<World<'a>, PsyParseError> {
    if tree.is_internal() {
        let background_color;
        let mut lights: Vec<&WorldLightSource> = Vec::new();

        // Parse background shader
        let bgs = {
            if tree.iter_children_with_type("BackgroundShader").count() != 1 {
                return Err(PsyParseError::UnknownError);
            }
            tree.iter_children_with_type("BackgroundShader").nth(0).unwrap()
        };
        let bgs_type = {
            if bgs.iter_children_with_type("Type").count() != 1 {
                return Err(PsyParseError::UnknownError);
            }
            if let &DataTree::Leaf { contents, .. } =
                bgs.iter_children_with_type("Type")
                    .nth(0)
                    .unwrap() {
                contents.trim()
            } else {
                return Err(PsyParseError::UnknownError);
            }
        };
        match bgs_type {
            "Color" => {
                if let Some(&DataTree::Leaf { contents, .. }) =
                    bgs.iter_children_with_type("Color")
                        .nth(0) {
                    if let IResult::Done(_, color) =
                        closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.trim()
                            .as_bytes()) {
                        // TODO: proper color space management, not just assuming
                        // rec.709.
                        background_color = XYZ::from_tuple(rec709e_to_xyz(color));
                    } else {
                        return Err(PsyParseError::UnknownError);
                    }
                } else {
                    return Err(PsyParseError::UnknownError);
                }
            }

            _ => return Err(PsyParseError::UnknownError),
        }

        // Parse light sources
        for child in tree.iter_children() {
            match child {
                &DataTree::Internal { type_name, .. } if type_name == "DistantDiskLight" => {
                    lights.push(arena.alloc(parse_distant_disk_light(arena, &child)?));
                }

                _ => {}
            }
        }

        // Build and return the world
        return Ok(World {
            background_color: background_color,
            lights: arena.copy_slice(&lights),
        });
    } else {
        return Err(PsyParseError::UnknownError);
    }
}




pub fn parse_matrix(contents: &str) -> Result<Matrix4x4, PsyParseError> {
    if let IResult::Done(_, ns) =
        closure!(terminated!(tuple!(ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32,
                                    ws_f32),
                             nom::eof))(contents.as_bytes()) {
        return Ok(Matrix4x4::new_from_values(ns.0,
                                             ns.4,
                                             ns.8,
                                             ns.12,
                                             ns.1,
                                             ns.5,
                                             ns.9,
                                             ns.13,
                                             ns.2,
                                             ns.6,
                                             ns.10,
                                             ns.14,
                                             ns.3,
                                             ns.7,
                                             ns.11,
                                             ns.15));
    } else {
        return Err(PsyParseError::UnknownError);
    }
}
