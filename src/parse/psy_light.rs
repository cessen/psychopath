#![allow(dead_code)]

use std::result::Result;

use nom::{combinator::all_consuming, sequence::tuple, IResult};

use kioku::Arena;

use crate::{
    light::{DistantDiskLight, RectangleLight, SphereLight},
    math::Vector,
};

use super::{
    basics::ws_f32,
    psy::{parse_color, PsyParseError},
    DataTree,
};

pub fn parse_distant_disk_light<'a>(
    arena: &'a Arena,
    tree: &'a DataTree,
) -> Result<DistantDiskLight<'a>, PsyParseError> {
    if let DataTree::Internal { ref children, .. } = *tree {
        let mut radii = Vec::new();
        let mut directions = Vec::new();
        let mut colors = Vec::new();

        // Parse
        for child in children.iter() {
            match *child {
                // Radius
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Radius" => {
                    if let IResult::Ok((_, radius)) = all_consuming(ws_f32)(contents) {
                        radii.push(radius);
                    } else {
                        // Found radius, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                // Direction
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Direction" => {
                    if let IResult::Ok((_, direction)) =
                        all_consuming(tuple((ws_f32, ws_f32, ws_f32)))(contents)
                    {
                        directions.push(Vector::new(direction.0, direction.1, direction.2));
                    } else {
                        // Found direction, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                // Color
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Color" => {
                    if let Ok(color) = parse_color(contents) {
                        colors.push(color);
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                _ => {}
            }
        }

        return Ok(DistantDiskLight::new(arena, &radii, &directions, &colors));
    } else {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }
}

pub fn parse_sphere_light<'a>(
    arena: &'a Arena,
    tree: &'a DataTree,
) -> Result<SphereLight<'a>, PsyParseError> {
    if let DataTree::Internal { ref children, .. } = *tree {
        let mut radii = Vec::new();
        let mut colors = Vec::new();

        // Parse
        for child in children.iter() {
            match *child {
                // Radius
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Radius" => {
                    if let IResult::Ok((_, radius)) = all_consuming(ws_f32)(contents) {
                        radii.push(radius);
                    } else {
                        // Found radius, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                // Color
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Color" => {
                    if let Ok(color) = parse_color(contents) {
                        colors.push(color);
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                _ => {}
            }
        }

        return Ok(SphereLight::new(arena, &radii, &colors));
    } else {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }
}

pub fn parse_rectangle_light<'a>(
    arena: &'a Arena,
    tree: &'a DataTree,
) -> Result<RectangleLight<'a>, PsyParseError> {
    if let DataTree::Internal { ref children, .. } = *tree {
        let mut dimensions = Vec::new();
        let mut colors = Vec::new();

        // Parse
        for child in children.iter() {
            match *child {
                // Dimensions
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Dimensions" => {
                    if let IResult::Ok((_, radius)) =
                        all_consuming(tuple((ws_f32, ws_f32)))(contents)
                    {
                        dimensions.push(radius);
                    } else {
                        // Found dimensions, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                // Color
                DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } if type_name == "Color" => {
                    if let Ok(color) = parse_color(contents) {
                        colors.push(color);
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                _ => {}
            }
        }

        return Ok(RectangleLight::new(arena, &dimensions, &colors));
    } else {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }
}
