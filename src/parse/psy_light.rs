#![allow(dead_code)]

use std::result::Result;

use nom::IResult;

use super::DataTree;
use super::basics::ws_f32;
use super::psy::PsyParseError;

use light::{SphereLight, RectangleLight};
use color::{XYZ, rec709e_to_xyz};

pub fn parse_sphere_light(tree: &DataTree) -> Result<SphereLight, PsyParseError> {
    if let &DataTree::Internal { ref children, .. } = tree {
        let mut radii = Vec::new();
        let mut colors = Vec::new();

        // Parse
        for child in children.iter() {
            match child {
                // Radius
                &DataTree::Leaf { type_name, contents } if type_name == "Radius" => {
                    if let IResult::Done(_, radius) = ws_f32(contents.as_bytes()) {
                        radii.push(radius);
                    } else {
                        // Found radius, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                // Color
                &DataTree::Leaf { type_name, contents } if type_name == "Color" => {
                    if let IResult::Done(_, color) = closure!(tuple!(ws_f32,
                                                                     ws_f32,
                                                                     ws_f32))(contents.as_bytes()) {
                        // TODO: handle color space conversions properly.
                        // Probably will need a special color type with its
                        // own parser...?
                        colors.push(XYZ::from_tuple(rec709e_to_xyz(color)));
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                _ => {}
            }
        }

        return Ok(SphereLight::new(radii, colors));
    } else {
        return Err(PsyParseError::UnknownError);
    }
}

pub fn parse_rectangle_light(tree: &DataTree) -> Result<RectangleLight, PsyParseError> {
    if let &DataTree::Internal { ref children, .. } = tree {
        let mut dimensions = Vec::new();
        let mut colors = Vec::new();

        // Parse
        for child in children.iter() {
            match child {
                // Dimensions
                &DataTree::Leaf { type_name, contents } if type_name == "Dimensions" => {
                    if let IResult::Done(_, radius) =
                           closure!(tuple!(ws_f32, ws_f32))(contents.as_bytes()) {
                        dimensions.push(radius);
                    } else {
                        // Found dimensions, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                // Color
                &DataTree::Leaf { type_name, contents } if type_name == "Color" => {
                    if let IResult::Done(_, color) = closure!(tuple!(ws_f32,
                                                                     ws_f32,
                                                                     ws_f32))(contents.as_bytes()) {
                        // TODO: handle color space conversions properly.
                        // Probably will need a special color type with its
                        // own parser...?
                        colors.push(XYZ::from_tuple(rec709e_to_xyz(color)));
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError);
                    }
                }

                _ => {}
            }
        }

        return Ok(RectangleLight::new(dimensions, colors));
    } else {
        return Err(PsyParseError::UnknownError);
    }
}
