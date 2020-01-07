#![allow(dead_code)]

use std::{collections::HashMap, f32, io::BufRead, result::Result};

use nom::{combinator::all_consuming, sequence::tuple, IResult};

use data_tree::{
    reader::{DataTreeReader, ReaderError},
    Event,
};
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
    ReaderError(ReaderError),
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

impl std::error::Error for PsyParseError {}

impl std::fmt::Display for PsyParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl From<ReaderError> for PsyParseError {
    fn from(e: ReaderError) -> Self {
        PsyParseError::ReaderError(e)
    }
}

fn line_count_to_byte_offset(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
}

//----------------------------------------------------------------

/// Takes in a `DataTree` representing a Scene node and returns
pub fn parse_scene<'a>(
    arena: &'a Arena,
    events: &mut DataTreeReader<impl BufRead>,
    ident: Option<&str>,
) -> Result<Scene<'a>, PsyParseError> {
    // Get output info.
    let output_info = if let Event::InnerOpen {
        type_name: "Output",
        ..
    } = events.next_event()?
    {
        parse_output_info(events)?
    } else {
        todo!(); // Return error.
    };

    // Get render settings.
    let render_settings = if let Event::InnerOpen {
        type_name: "RenderSettings",
        ..
    } = events.next_event()?
    {
        parse_render_settings(events)?
    } else {
        todo!(); // Return error.
    };

    // Get camera.
    let camera = if let Event::InnerOpen {
        type_name: "Camera",
        ..
    } = events.next_event()?
    {
        parse_camera(arena, events)?
    } else {
        todo!(); // Return error.
    };

    // Get shaders.
    let shaders = if let Event::InnerOpen {
        type_name: "Shaders",
        ..
    } = events.next_event()?
    {
        parse_shaders(arena, events)?
    } else {
        todo!(); // Return error.
    };

    // Get world.
    let world = if let Event::InnerOpen {
        type_name: "World", ..
    } = events.next_event()?
    {
        parse_world(arena, events)?
    } else {
        todo!(); // Return error.
    };

    // Get the root assembly.
    let root_assembly = if let Event::InnerOpen {
        type_name: "Assembly",
        ident,
        ..
    } = events.next_event()?
    {
        parse_assembly(arena, events, &(ident.into::<String>()))?
    } else {
        todo!(); // Return error.
    };

    // Make sure we're closed out properly.
    if let Event::InnerClose { .. } = events.next_event()? {
    } else {
        todo!(); // Return error.
    }

    // Put scene together
    let scene_name = if let Some(name) = ident {
        Some(name.into())
    } else {
        None
    };

    let scene = Scene {
        camera: camera,
        world: world,
        shaders: shaders,
        root_assembly: root_assembly,
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

fn parse_output_info(events: &mut DataTreeReader<impl BufRead>) -> Result<String, PsyParseError> {
    let mut found_path = false;
    let mut path = String::new();
    loop {
        match events.next_event()? {
            Event::Leaf {
                type_name: "Path",
                contents,
                byte_offset,
            } => {
                // Trim and validate
                let tc = contents.trim();
                if tc.chars().count() < 2 {
                    return Err(PsyParseError::IncorrectLeafData(
                        *byte_offset,
                        "File path format is incorrect.",
                    ));
                }
                if tc.chars().nth(0).unwrap() != '"' || !tc.ends_with('"') {
                    return Err(PsyParseError::IncorrectLeafData(
                        *byte_offset,
                        "File paths must be surrounded by quotes.",
                    ));
                }
                let len = tc.len();
                let tc = &tc[1..len - 1];

                // Parse
                // TODO: proper string escaping
                found_path = true;
                path = tc.to_string();
            }

            Event::InnerClose { .. } => {
                break;
            }

            _ => {
                todo!(); // Return error.
            }
        }
    }

    if found_path {
        return Ok(path);
    } else {
        // return Err(PsyParseError::MissingNode(
        //     tree.byte_offset(),
        //     "Output section must contain a Path.",
        // ));
        todo!(); // Return error.
    }
}

fn parse_render_settings(
    events: &mut DataTreeReader<impl BufRead>,
) -> Result<((u32, u32), u32, u32), PsyParseError> {
    let mut found_res = false;
    let mut found_spp = false;
    let mut res = (0, 0);
    let mut spp = 0;
    let mut seed = 0;
    loop {
        match events.next_event()? {
            // Resolution
            Event::Leaf {
                type_name: "Resolution",
                contents,
                byte_offset,
            } => {
                if let IResult::Ok((_, (w, h))) = all_consuming(tuple((ws_u32, ws_u32)))(&contents)
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
                type_name: "SamplesPerPixel",
                contents,
                byte_offset,
            } => {
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
                type_name: "Seed",
                contents,
                byte_offset,
            } => {
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

            Event::InnerClose { .. } => {
                break;
            }

            _ => {
                todo!(); // Return error.
            }
        }
    }

    if found_res && found_spp {
        return Ok((res, spp, seed));
    } else {
        // return Err(PsyParseError::MissingNode(
        //     tree.byte_offset(),
        //     "RenderSettings must have both Resolution and \
        //      SamplesPerPixel specified.",
        // ));
        todo!(); // Return error.
    }
}

fn parse_camera<'a>(
    arena: &'a Arena,
    events: &mut DataTreeReader<impl BufRead>,
) -> Result<Camera<'a>, PsyParseError> {
    let mut mats = Vec::new();
    let mut fovs = Vec::new();
    let mut focus_distances = Vec::new();
    let mut aperture_radii = Vec::new();

    // Parse
    loop {
        match events.next_event()? {
            // Fov
            Event::Leaf {
                type_name: "Fov",
                contents,
                byte_offset,
            } => {
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
            Event::Leaf {
                type_name: "FocalDistance",
                contents,
                byte_offset,
            } => {
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
            Event::Leaf {
                type_name: "ApertureRadius",
                contents,
                byte_offset,
            } => {
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
            Event::Leaf {
                type_name: "Transform",
                contents,
                byte_offset,
            } => {
                if let Ok(mat) = parse_matrix(&contents) {
                    mats.push(mat);
                } else {
                    // Found Transform, but its contents is not in the right format
                    return Err(make_transform_format_error(*byte_offset));
                }
            }

            Event::InnerClose { .. } => {
                break;
            }

            _ => {
                todo!(); // Return error.
            }
        }
    }

    return Ok(Camera::new(
        arena,
        &mats,
        &fovs,
        &aperture_radii,
        &focus_distances,
    ));
}

fn parse_world<'a>(
    arena: &'a Arena,
    events: &mut DataTreeReader<impl BufRead>,
) -> Result<World<'a>, PsyParseError> {
    let background_color = None;
    let mut lights: Vec<&dyn WorldLightSource> = Vec::new();

    loop {
        match events.next_event()? {
            // Parse background shader
            Event::InnerOpen {
                type_name: "BackgroundShader",
            } => {
                let bgs_type = if let Event::Leaf {
                    type_name: "Type",
                    contents,
                    ..
                } = events.next_event()
                {
                    contents.into::<String>()
                } else {
                    todo!(); // Return error.
                };

                match bgs_type {
                    "Color" => {
                        if let Event::Leaf {
                            type_name: "Color",
                            contents,
                            ..
                        } = events.next_event()
                        {
                            background_color = Some(parse_color(contents)?);
                        } else {
                            todo!(
                                "BackgroundShader's Type is Color, \
                                 but no Color is specified."
                            ); // Return error.
                        };
                    }

                    _ => {
                        todo!(
                            "The specified BackgroundShader Type \
                             isn't a recognized type.",
                        ); // Return an error.
                    }
                }

                // Close it out.
                if let Event::InnerClose { .. } = events.next_event() {
                } else {
                    todo!(); // Return error.
                }
            }

            // Parse light sources
            Event::InnerOpen {
                type_name: "DistantDiskLight",
                ..
            } => {
                lights.push(arena.alloc(parse_distant_disk_light(arena, events)?));
            }
        }
    }

    if background_color == None {
        todo!(); // Return error.
    }

    // Build and return the world
    return Ok(World {
        background_color: background_color.unwrap(),
        lights: arena.copy_slice(&lights),
    });
}

fn parse_shaders<'a>(
    events: &mut DataTreeReader<impl BufRead>,
) -> Result<HashMap<String, Box<dyn SurfaceShader>>, PsyParseError> {
    let mut shaders = HashMap::new();
    loop {
        match events.next_event() {
            DataTree::Internal {
                type_name: "SurfaceShader",
                ident,
                byte_offset,
            } => {
                if let Some(name) = ident {
                    let name = name.to_string();
                    shaders.insert(name, parse_surface_shader(events)?);
                } else {
                    todo!("Shader has no name."); // Return error.
                }
            }

            _ => {
                todo!(); // Return error.
            }
        }
    }

    // Return the list of shaders.
    return Ok(shaders);
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

    todo!(); // Return an error.
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
        todo!(); // Return an error.
    }

    match items[0] {
        "rec709" => {
            if let IResult::Ok((_, color)) = tuple((ws_f32, ws_f32, ws_f32))(items[1]) {
                return Ok(Color::new_xyz(rec709_e_to_xyz(color)));
            } else {
                todo!(); // Return an error.
            }
        }

        "blackbody" => {
            if let IResult::Ok((_, (temperature, factor))) = tuple((ws_f32, ws_f32))(items[1]) {
                return Ok(Color::new_blackbody(temperature, factor));
            } else {
                todo!(); // Return an error.
            }
        }

        "color_temperature" => {
            if let IResult::Ok((_, (temperature, factor))) = tuple((ws_f32, ws_f32))(items[1]) {
                return Ok(Color::new_temperature(temperature, factor));
            } else {
                todo!(); // Return an error.
            }
        }

        _ => {
            todo!(); // Return an error.
        }
    }
}
