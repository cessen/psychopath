#![allow(dead_code)]

use std::result::Result;

use nom;
use nom::IResult;

use renderer::Renderer;
use super::DataTree;
use super::basics::ws_u32;


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
    let output_info = if let &DataTree::Internal{ref children, ..} =
                             tree.get_first_child_with_type_name("Output").unwrap() {
        let mut found_path = false;
        let mut path = String::new();

        for child in children {
            match child {
                &DataTree::Leaf { type_name, contents } if type_name == "Path" => {
                    // TODO: proper string escaping and quotes stripping
                    found_path = true;
                    path = contents.to_string();
                }

                &DataTree::Leaf { type_name, contents } if type_name == "FileFormat" => {
                    // TODO
                    unimplemented!()
                }

                &DataTree::Leaf { type_name, contents } if type_name == "ColorSpace" => {
                    // TODO
                    unimplemented!()
                }

                &DataTree::Leaf { type_name, contents } if type_name == "Dither" => {
                    // TODO
                    unimplemented!()
                }

                _ => {}
            }
        }

        if found_path {
            (path)
        } else {
            return Err(());
        }
    } else {
        return Err(());
    };

    // Parse render settings
    let render_settings = if let &DataTree::Internal{ref children, ..} =
                                 tree.get_first_child_with_type_name("RenderSettings").unwrap() {
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

                &DataTree::Leaf { type_name, contents } if type_name == "SamplesPerPixel" => {
                    if let IResult::Done(_, n) = ws_u32(contents.as_bytes()) {
                        found_spp = true;
                        spp = n;
                    } else {
                        // Found SamplesPerPixel, but its contents is not in the right format
                        return Err(());
                    }
                }

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
            (res, spp)
        } else {
            return Err(());
        }
    } else {
        return Err(());
    };

    // Parse camera

    // Parse world

    // Parse root scene assembly

    return Err(());
}
