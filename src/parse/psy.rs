#![allow(dead_code)]

use std::result::Result;

use nom;
use nom::IResult;

use super::DataTree;
use super::basics::{ws_u32, ws_f32};
use super::psy_assembly::parse_assembly;

use math::Matrix4x4;
use camera::Camera;
use renderer::Renderer;
use scene::Scene;




/// Takes in a DataTree representing a Scene node and returns
/// a renderer.
pub fn parse_frame(tree: &DataTree) -> Result<Renderer, ()> {
    // Verify we have the right number of each section
    if tree.count_children_with_type_name("Output") != 1 {
        return Err(());
    }
    if tree.count_children_with_type_name("RenderSettings") != 1 {
        return Err(());
    }
    if tree.count_children_with_type_name("Camera") != 1 {
        return Err(());
    }
    if tree.count_children_with_type_name("World") != 1 {
        return Err(());
    }
    if tree.count_children_with_type_name("Assembly") != 1 {
        return Err(());
    }

    // Parse output info
    let output_info = try!(parse_output_info(tree.get_first_child_with_type_name("Output")
                                                 .unwrap()));

    // Parse render settings
    let render_settings = try!(parse_render_settings(tree.get_first_child_with_type_name("Rende\
                                                                                          rSett\
                                                                                          ings")
                                                         .unwrap()));

    // Parse camera
    let camera = try!(parse_camera(tree.get_first_child_with_type_name("Camera").unwrap()));

    // Parse world
    let world = try!(parse_world(tree.get_first_child_with_type_name("World").unwrap()));

    // Parse root scene assembly
    let assembly = try!(parse_assembly(tree.get_first_child_with_type_name("Assembly").unwrap()));

    // Put scene together
    let scene_name = if let &DataTree::Internal{ident, ..} = tree {
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
        background_color: world,
        camera: camera,
        root: assembly,
    };

    // // Put renderer together
    // let renderer = Renderer {
    //     output_file: output_info.0.clone(),
    //     resolution: (render_settings.0.0 as usize, render_settings.0.1 as usize),
    //     spp: render_settings.1,
    //     scene: scene,
    // }
    //
    // return Ok(renderer);

    return Err(());
}




fn parse_output_info(tree: &DataTree) -> Result<(String), ()> {
    if let &DataTree::Internal{ref children, ..} = tree {
        let mut found_path = false;
        let mut path = String::new();

        for child in children {
            match child {
                &DataTree::Leaf { type_name, contents } if type_name == "Path" => {
                    // TODO: proper string escaping and quotes stripping
                    found_path = true;
                    path = contents.to_string();
                }

                _ => {}
            }
        }

        if found_path {
            return Ok((path));
        } else {
            return Err(());
        }
    } else {
        return Err(());
    };
}




fn parse_render_settings(tree: &DataTree) -> Result<((u32, u32), u32, u32), ()> {
    if let &DataTree::Internal{ref children, ..} = tree {
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
                        return Err(());
                    }
                }

                // SamplesPerPixel
                &DataTree::Leaf { type_name, contents } if type_name == "SamplesPerPixel" => {
                    if let IResult::Done(_, n) = ws_u32(contents.as_bytes()) {
                        found_spp = true;
                        spp = n;
                    } else {
                        // Found SamplesPerPixel, but its contents is not in the right format
                        return Err(());
                    }
                }

                // Seed
                &DataTree::Leaf { type_name, contents } if type_name == "Seed" => {
                    if let IResult::Done(_, n) = ws_u32(contents.as_bytes()) {
                        seed = n;
                    } else {
                        // Found Seed, but its contents is not in the right format
                        return Err(());
                    }
                }

                _ => {}
            }
        }

        if found_res && found_spp {
            return Ok((res, spp, seed));
        } else {
            return Err(());
        }
    } else {
        return Err(());
    };
}




fn parse_camera(tree: &DataTree) -> Result<Camera, ()> {
    if let &DataTree::Internal{ref children, ..} = tree {
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
                        return Err(());
                    }
                }

                // FocalDistance
                &DataTree::Leaf { type_name, contents } if type_name == "FocalDistance" => {
                    if let IResult::Done(_, fd) = ws_f32(contents.as_bytes()) {
                        focus_distances.push(fd);
                    } else {
                        // Found FocalDistance, but its contents is not in the right format
                        return Err(());
                    }
                }

                // ApertureRadius
                &DataTree::Leaf { type_name, contents } if type_name == "ApertureRadius" => {
                    if let IResult::Done(_, ar) = ws_f32(contents.as_bytes()) {
                        aperture_radii.push(ar);
                    } else {
                        // Found ApertureRadius, but its contents is not in the right format
                        return Err(());
                    }
                }

                // Transform
                &DataTree::Leaf { type_name, contents } if type_name == "Transform" => {
                    if let Ok(mat) = parse_matrix(contents) {
                        mats.push(mat);
                    } else {
                        // Found Transform, but its contents is not in the right format
                        return Err(());
                    }
                }

                _ => {}
            }
        }

        return Ok(Camera::new(mats, fovs, aperture_radii, focus_distances));
    } else {
        return Err(());
    }
}




fn parse_world(tree: &DataTree) -> Result<(f32, f32, f32), ()> {
    if tree.is_internal() {
        let mut found_background_color = false;
        let mut background_color = (0.0, 0.0, 0.0);

        // Parse background shader
        let bgs = {
            if tree.count_children_with_type_name("BackgroundShader") != 1 {
                return Err(());
            }
            tree.get_first_child_with_type_name("BackgroundShader").unwrap()
        };
        let bgs_type = {
            if bgs.count_children_with_type_name("Type") != 1 {
                return Err(());
            }
            if let &DataTree::Leaf{contents, ..} = tree.get_first_child_with_type_name("Type")
                                                       .unwrap() {
                contents.trim()
            } else {
                return Err(());
            }
        };
        match bgs_type {
            "Color" => {
                if let Some(&DataTree::Leaf{contents, ..}) =
                       bgs.get_first_child_with_type_name("Color") {
                    if let IResult::Done(_, color) = closure!(tuple!(ws_f32,
                                                                     ws_f32,
                                                                     ws_f32))(contents.trim()
                                                                                      .as_bytes()) {
                        found_background_color = true;
                        background_color = color;
                    } else {
                        return Err(());
                    }
                } else {
                    return Err(());
                }
            }

            _ => return Err(()),
        }

        return Ok(background_color);
    } else {
        return Err(());
    }
}




fn parse_matrix(contents: &str) -> Result<Matrix4x4, ()> {
    if let IResult::Done(_, ns) = closure!(terminated!(tuple!(ws_f32,
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
                                             ns.1,
                                             ns.2,
                                             ns.3,
                                             ns.4,
                                             ns.5,
                                             ns.6,
                                             ns.7,
                                             ns.8,
                                             ns.9,
                                             ns.10,
                                             ns.11,
                                             ns.12,
                                             ns.13,
                                             ns.14,
                                             ns.15));
    } else {
        return Err(());
    }
}
