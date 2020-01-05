#![allow(dead_code)]

use std::{collections::HashMap, f32, result::Result};

use nom::{combinator::all_consuming, sequence::tuple, IResult};

use kioku::Arena;

use crate::{
    camera::Camera,
    color::{rec709_e_to_xyz, Color},
    light::WorldLightSource,
    math::Matrix4x4,
    // renderer::Renderer,
    scene::Scene,
    scene::World,
    shading::SurfaceShader,
};

use super::{
    basics::{ws_f32, ws_u32},
    psy_assembly::parse_assembly,
    psy_light::parse_distant_disk_light,
    psy_surface_shader::parse_surface_shader,
    DataTree,
};

#[derive(Debug)]
pub enum PsyParseError {
    // The first usize for all errors is their byte offset
    // into the psy content where they occured.
    UnknownError(usize),
    UnknownVariant(usize, &'static str),        // Error message
    ExpectedInternalNode(usize, &'static str),  // Error message
    ExpectedLeafNode(usize, &'static str),      // Error message
    ExpectedIdent(usize, &'static str),         // Error message
    MissingNode(usize, &'static str),           // Error message
    IncorrectLeafData(usize, &'static str),     // Error message
    WrongNodeCount(usize, &'static str, usize), // Error message, sections found
    InstancedMissingData(usize, &'static str, String), // Error message, data name
}

impl PsyParseError {
    pub fn print(&self, psy_content: &str) {
        match *self {
            PsyParseError::UnknownError(offset) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!(
                    "Line {}: Unknown parse error.  If you get this message, please report \
                     it to the developers so they can improve the error messages.",
                    line
                );
            }

            PsyParseError::UnknownVariant(offset, error) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {}", line, error);
            }

            PsyParseError::ExpectedInternalNode(offset, error) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {}", line, error);
            }

            PsyParseError::ExpectedLeafNode(offset, error) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {}", line, error);
            }

            PsyParseError::ExpectedIdent(offset, error) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {}", line, error);
            }

            PsyParseError::MissingNode(offset, error) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {}", line, error);
            }

            PsyParseError::IncorrectLeafData(offset, error) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {}", line, error);
            }

            PsyParseError::WrongNodeCount(offset, error, count) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {}  Found: {}", line, error, count);
            }

            PsyParseError::InstancedMissingData(offset, error, ref data_name) => {
                let line = line_count_to_byte_offset(psy_content, offset);
                println!("Line {}: {} Data name: '{}'", line, error, data_name);
            }
        }
    }
}

fn line_count_to_byte_offset(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
}

/// Takes in a `DataTree` representing a Scene node and returns
pub fn parse_scene<'a>(arena: &'a Arena, tree: &'a DataTree) -> Result<Scene<'a>, PsyParseError> {
    // Verify we have the right number of each section
    if tree.iter_children_with_type("Output").count() != 1 {
        let count = tree.iter_children_with_type("Output").count();
        return Err(PsyParseError::WrongNodeCount(
            tree.byte_offset(),
            "Scene should have precisely one Output \
             section.",
            count,
        ));
    }
    if tree.iter_children_with_type("RenderSettings").count() != 1 {
        let count = tree.iter_children_with_type("RenderSettings").count();
        return Err(PsyParseError::WrongNodeCount(
            tree.byte_offset(),
            "Scene should have precisely one \
             RenderSettings section.",
            count,
        ));
    }
    if tree.iter_children_with_type("Camera").count() != 1 {
        let count = tree.iter_children_with_type("Camera").count();
        return Err(PsyParseError::WrongNodeCount(
            tree.byte_offset(),
            "Scene should have precisely one Camera \
             section.",
            count,
        ));
    }
    if tree.iter_children_with_type("World").count() != 1 {
        let count = tree.iter_children_with_type("World").count();
        return Err(PsyParseError::WrongNodeCount(
            tree.byte_offset(),
            "Scene should have precisely one World section.",
            count,
        ));
    }
    if tree.iter_children_with_type("Shaders").count() != 1 {
        let count = tree.iter_children_with_type("Shaders").count();
        return Err(PsyParseError::WrongNodeCount(
            tree.byte_offset(),
            "Scene should have precisely one Shaders section.",
            count,
        ));
    }
    if tree.iter_children_with_type("Assembly").count() != 1 {
        let count = tree.iter_children_with_type("Assembly").count();
        return Err(PsyParseError::WrongNodeCount(
            tree.byte_offset(),
            "Scene should have precisely one Root Assembly \
             section.",
            count,
        ));
    }

    // Parse output info
    let output_info = parse_output_info(tree.iter_children_with_type("Output").nth(0).unwrap())?;

    // Parse render settings
    let render_settings = parse_render_settings(
        tree.iter_children_with_type("RenderSettings")
            .nth(0)
            .unwrap(),
    )?;

    // Parse camera
    let camera = parse_camera(
        arena,
        tree.iter_children_with_type("Camera").nth(0).unwrap(),
    )?;

    // Parse world
    let world = parse_world(arena, tree.iter_children_with_type("World").nth(0).unwrap())?;

    // Parse shaders
    let shaders = parse_shaders(tree.iter_children_with_type("Shaders").nth(0).unwrap())?;

    // Parse root scene assembly
    let assembly = parse_assembly(
        arena,
        tree.iter_children_with_type("Assembly").nth(0).unwrap(),
    )?;

    // Put scene together
    let scene_name = if let DataTree::Internal { ident, .. } = tree {
        if let Some(name) = ident {
            Some(name.clone())
        } else {
            None
        }
    } else {
        None
    };
    let scene = Scene {
        camera: camera,
        world: world,
        shaders: shaders,
        root_assembly: assembly,
    };

    // // Put renderer together
    // let renderer = Renderer {
    //     output_file: output_info.clone(),
    //     resolution: (
    //         (render_settings.0).0 as usize,
    //         (render_settings.0).1 as usize,
    //     ),
    //     spp: render_settings.1 as usize,
    //     seed: render_settings.2,
    //     scene: scene,
    // };

    return Ok(scene);
}

fn parse_output_info(tree: &DataTree) -> Result<String, PsyParseError> {
    if let DataTree::Internal { ref children, .. } = *tree {
        let mut found_path = false;
        let mut path = String::new();

        for child in children {
            match child {
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Path" => {
                    // Trim and validate
                    let tc = contents.trim();
                    if tc.chars().count() < 2 {
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "File path format is \
                             incorrect.",
                        ));
                    }
                    if tc.chars().nth(0).unwrap() != '"' || !tc.ends_with('"') {
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "File paths must be \
                             surrounded by quotes.",
                        ));
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
            return Ok(path);
        } else {
            return Err(PsyParseError::MissingNode(
                tree.byte_offset(),
                "Output section must contain a Path.",
            ));
        }
    } else {
        return Err(PsyParseError::ExpectedInternalNode(
            tree.byte_offset(),
            "Output section should be an internal \
             node, containing at least a Path.",
        ));
    };
}

fn parse_render_settings(tree: &DataTree) -> Result<((u32, u32), u32, u32), PsyParseError> {
    if let DataTree::Internal { ref children, .. } = *tree {
        let mut found_res = false;
        let mut found_spp = false;
        let mut res = (0, 0);
        let mut spp = 0;
        let mut seed = 0;

        for child in children {
            match child {
                // Resolution
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Resolution" => {
                    if let IResult::Ok((_, (w, h))) =
                        all_consuming(tuple((ws_u32, ws_u32)))(&contents)
                    {
                        found_res = true;
                        res = (w, h);
                    } else {
                        // Found Resolution, but its contents is not in the right format
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "Resolution should be specified with two \
                             integers in the form '[width height]'.",
                        ));
                    }
                }

                // SamplesPerPixel
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "SamplesPerPixel" => {
                    if let IResult::Ok((_, n)) = all_consuming(ws_u32)(&contents) {
                        found_spp = true;
                        spp = n;
                    } else {
                        // Found SamplesPerPixel, but its contents is not in the right format
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "SamplesPerPixel should be \
                             an integer specified in \
                             the form '[samples]'.",
                        ));
                    }
                }

                // Seed
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Seed" => {
                    if let IResult::Ok((_, n)) = all_consuming(ws_u32)(&contents) {
                        seed = n;
                    } else {
                        // Found Seed, but its contents is not in the right format
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "Seed should be an integer \
                             specified in the form \
                             '[samples]'.",
                        ));
                    }
                }

                _ => {}
            }
        }

        if found_res && found_spp {
            return Ok((res, spp, seed));
        } else {
            return Err(PsyParseError::MissingNode(
                tree.byte_offset(),
                "RenderSettings must have both Resolution and \
                 SamplesPerPixel specified.",
            ));
        }
    } else {
        return Err(PsyParseError::ExpectedInternalNode(
            tree.byte_offset(),
            "RenderSettings section should be an \
             internal node, containing at least \
             Resolution and SamplesPerPixel.",
        ));
    };
}

fn parse_camera<'a>(arena: &'a Arena, tree: &'a DataTree) -> Result<Camera<'a>, PsyParseError> {
    if let DataTree::Internal { ref children, .. } = *tree {
        let mut mats = Vec::new();
        let mut fovs = Vec::new();
        let mut focus_distances = Vec::new();
        let mut aperture_radii = Vec::new();

        // Parse
        for child in children.iter() {
            match child {
                // Fov
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Fov" => {
                    if let IResult::Ok((_, fov)) = all_consuming(ws_f32)(&contents) {
                        fovs.push(fov * (f32::consts::PI / 180.0));
                    } else {
                        // Found Fov, but its contents is not in the right format
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "Fov should be a decimal \
                             number specified in the \
                             form '[fov]'.",
                        ));
                    }
                }

                // FocalDistance
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "FocalDistance" => {
                    if let IResult::Ok((_, fd)) = all_consuming(ws_f32)(&contents) {
                        focus_distances.push(fd);
                    } else {
                        // Found FocalDistance, but its contents is not in the right format
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "FocalDistance should be a \
                             decimal number specified \
                             in the form '[fov]'.",
                        ));
                    }
                }

                // ApertureRadius
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "ApertureRadius" => {
                    if let IResult::Ok((_, ar)) = all_consuming(ws_f32)(&contents) {
                        aperture_radii.push(ar);
                    } else {
                        // Found ApertureRadius, but its contents is not in the right format
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "ApertureRadius should be a \
                             decimal number specified \
                             in the form '[fov]'.",
                        ));
                    }
                }

                // Transform
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Transform" => {
                    if let Ok(mat) = parse_matrix(&contents) {
                        mats.push(mat);
                    } else {
                        // Found Transform, but its contents is not in the right format
                        return Err(make_transform_format_error(*byte_offset));
                    }
                }

                _ => {}
            }
        }

        return Ok(Camera::new(
            arena,
            &mats,
            &fovs,
            &aperture_radii,
            &focus_distances,
        ));
    } else {
        return Err(PsyParseError::ExpectedInternalNode(
            tree.byte_offset(),
            "Camera section should be an internal \
             node, containing at least Fov and \
             Transform.",
        ));
    }
}

fn parse_world<'a>(arena: &'a Arena, tree: &'a DataTree) -> Result<World<'a>, PsyParseError> {
    if tree.is_internal() {
        let background_color;
        let mut lights: Vec<&dyn WorldLightSource> = Vec::new();

        // Parse background shader
        let bgs = {
            if tree.iter_children_with_type("BackgroundShader").count() != 1 {
                return Err(PsyParseError::WrongNodeCount(
                    tree.byte_offset(),
                    "World should have precisely one BackgroundShader section.",
                    tree.iter_children_with_type("BackgroundShader").count(),
                ));
            }
            tree.iter_children_with_type("BackgroundShader")
                .nth(0)
                .unwrap()
        };
        let bgs_type = {
            if bgs.iter_children_with_type("Type").count() != 1 {
                return Err(PsyParseError::WrongNodeCount(
                    bgs.byte_offset(),
                    "BackgroundShader should have \
                     precisely one Type specified.",
                    bgs.iter_children_with_type("Type").count(),
                ));
            }
            if let DataTree::Leaf { contents, .. } =
                bgs.iter_children_with_type("Type").nth(0).unwrap()
            {
                contents.trim()
            } else {
                return Err(PsyParseError::ExpectedLeafNode(
                    bgs.byte_offset(),
                    "BackgroundShader's Type should be a \
                     leaf node.",
                ));
            }
        };
        match bgs_type {
            "Color" => {
                if let Some(DataTree::Leaf {
                    contents,
                    byte_offset,
                    ..
                }) = bgs.iter_children_with_type("Color").nth(0)
                {
                    if let Ok(color) = parse_color(&contents) {
                        background_color = color;
                    } else {
                        return Err(PsyParseError::IncorrectLeafData(
                            *byte_offset,
                            "Color should be specified \
                             with three decimal numbers \
                             in the form '[R G B]'.",
                        ));
                    }
                } else {
                    return Err(PsyParseError::MissingNode(
                        bgs.byte_offset(),
                        "BackgroundShader's Type is Color, \
                         but no Color is specified.",
                    ));
                }
            }

            _ => {
                return Err(PsyParseError::UnknownVariant(
                    bgs.byte_offset(),
                    "The specified BackgroundShader Type \
                     isn't a recognized type.",
                ))
            }
        }

        // Parse light sources
        for child in tree.iter_children() {
            match child {
                DataTree::Internal { type_name, .. } if type_name == "DistantDiskLight" => {
                    lights.push(arena.alloc(parse_distant_disk_light(arena, child)?));
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
        return Err(PsyParseError::ExpectedInternalNode(
            tree.byte_offset(),
            "World section should be an internal \
             node, containing at least a \
             BackgroundShader.",
        ));
    }
}

fn parse_shaders<'a>(
    tree: &'a DataTree,
) -> Result<HashMap<String, Box<dyn SurfaceShader>>, PsyParseError> {
    if tree.is_internal() {
        let mut shaders = HashMap::new();

        for shader_item in tree.iter_children() {
            match shader_item {
                DataTree::Internal {
                    type_name,
                    ident,
                    children,
                    byte_offset,
                } if type_name == &"SurfaceShader" => {
                    if let Some(name) = ident {
                        shaders.insert(name.to_string(), parse_surface_shader(shader_item)?);
                    } else {
                        // TODO: error.
                    }
                }

                _ => {
                    // TODO: an error.
                }
            }
        }

        // Return the list of shaders.
        return Ok(shaders);
    } else {
        return Err(PsyParseError::ExpectedInternalNode(
            tree.byte_offset(),
            "Shaders section should be an internal \
             node.",
        ));
    }
}

pub fn parse_matrix(contents: &str) -> Result<Matrix4x4, PsyParseError> {
    if let IResult::Ok((leftover, ns)) = all_consuming(tuple((
        ws_f32, ws_f32, ws_f32, ws_f32, ws_f32, ws_f32, ws_f32, ws_f32, ws_f32, ws_f32, ws_f32,
        ws_f32, ws_f32, ws_f32, ws_f32, ws_f32,
    )))(contents)
    {
        if leftover.is_empty() {
            return Ok(Matrix4x4::new_from_values(
                ns.0, ns.4, ns.8, ns.12, ns.1, ns.5, ns.9, ns.13, ns.2, ns.6, ns.10, ns.14, ns.3,
                ns.7, ns.11, ns.15,
            ));
        }
    }

    return Err(PsyParseError::UnknownError(0));
}

pub fn make_transform_format_error(byte_offset: usize) -> PsyParseError {
    PsyParseError::IncorrectLeafData(
        byte_offset,
        "Transform should be sixteen integers specified in \
         the form '[# # # # # # # # # # # # # # # #]'.",
    )
}

pub fn parse_color(contents: &str) -> Result<Color, PsyParseError> {
    let items: Vec<_> = contents.split(',').map(|s| s.trim()).collect();
    if items.len() != 2 {
        return Err(PsyParseError::UnknownError(0));
    }

    match items[0] {
        "rec709" => {
            if let IResult::Ok((_, color)) = tuple((ws_f32, ws_f32, ws_f32))(items[1]) {
                return Ok(Color::new_xyz(rec709_e_to_xyz(color)));
            } else {
                return Err(PsyParseError::UnknownError(0));
            }
        }

        "blackbody" => {
            if let IResult::Ok((_, (temperature, factor))) = tuple((ws_f32, ws_f32))(items[1]) {
                return Ok(Color::new_blackbody(temperature, factor));
            } else {
                return Err(PsyParseError::UnknownError(0));
            }
        }

        "color_temperature" => {
            if let IResult::Ok((_, (temperature, factor))) = tuple((ws_f32, ws_f32))(items[1]) {
                return Ok(Color::new_temperature(temperature, factor));
            } else {
                return Err(PsyParseError::UnknownError(0));
            }
        }

        _ => return Err(PsyParseError::UnknownError(0)),
    }
}
