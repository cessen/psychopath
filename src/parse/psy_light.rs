#![allow(dead_code)]

use std::result::Result;

use nom::IResult;

use mem_arena::MemArena;

use color::{rec709_e_to_xyz, XYZ};
use light::{DistantDiskLight, RectangleLight, SphereLight};
use math::Vector;

use super::basics::ws_f32;
use super::psy::PsyParseError;
use super::DataTree;

pub fn parse_distant_disk_light<'a>(
    arena: &'a MemArena,
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
                } if type_name == "Radius" =>
                {
                    if let IResult::Done(_, radius) = ws_f32(contents.as_bytes()) {
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
                } if type_name == "Direction" =>
                {
                    if let IResult::Done(_, direction) =
                        closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.as_bytes())
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
                } if type_name == "Color" =>
                {
                    if let IResult::Done(_, color) =
                        closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.as_bytes())
                    {
                        // TODO: handle color space conversions properly.
                        // Probably will need a special color type with its
                        // own parser...?
                        colors.push(XYZ::from_tuple(rec709_e_to_xyz(color)));
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                _ => {}
            }
        }

        return Ok(DistantDiskLight::new(arena, radii, directions, colors));
    } else {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }
}

pub fn parse_sphere_light<'a>(
    arena: &'a MemArena,
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
                } if type_name == "Radius" =>
                {
                    if let IResult::Done(_, radius) = ws_f32(contents.as_bytes()) {
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
                } if type_name == "Color" =>
                {
                    if let IResult::Done(_, color) =
                        closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.as_bytes())
                    {
                        // TODO: handle color space conversions properly.
                        // Probably will need a special color type with its
                        // own parser...?
                        colors.push(XYZ::from_tuple(rec709_e_to_xyz(color)));
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                _ => {}
            }
        }

        return Ok(SphereLight::new(arena, radii, colors));
    } else {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }
}

pub fn parse_rectangle_light<'a>(
    arena: &'a MemArena,
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
                } if type_name == "Dimensions" =>
                {
                    if let IResult::Done(_, radius) =
                        closure!(tuple!(ws_f32, ws_f32))(contents.as_bytes())
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
                } if type_name == "Color" =>
                {
                    if let IResult::Done(_, color) =
                        closure!(tuple!(ws_f32, ws_f32, ws_f32))(contents.as_bytes())
                    {
                        // TODO: handle color space conversions properly.
                        // Probably will need a special color type with its
                        // own parser...?
                        colors.push(XYZ::from_tuple(rec709_e_to_xyz(color)));
                    } else {
                        // Found color, but its contents is not in the right format
                        return Err(PsyParseError::UnknownError(byte_offset));
                    }
                }

                _ => {}
            }
        }

        return Ok(RectangleLight::new(arena, dimensions, colors));
    } else {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }
}
