#![allow(dead_code)]

use std::result::Result;

use super::DataTree;
use super::psy::{parse_matrix, PsyParseError};
use super::psy_mesh_surface::parse_mesh_surface;

use assembly::{Assembly, AssemblyBuilder, Object};

pub fn parse_assembly(tree: &DataTree) -> Result<Assembly, PsyParseError> {
    let mut builder = AssemblyBuilder::new();

    if tree.is_internal() {
        for child in tree.iter_children() {
            match child.type_name() {
                // Sub-Assembly
                "Assembly" => {
                    if let &DataTree::Internal {ident: Some(ident), ..} = child {
                        builder.add_assembly(ident, try!(parse_assembly(&child)));
                    } else {
                        // TODO: error condition of some kind, because no ident
                        panic!();
                    }
                }

                // Instance
                "Instance" => {
                    // Pre-conditions
                    if !child.is_internal() {
                        // TODO: proper error
                        panic!();
                    }

                    // Get data name
                    let name = {
                        if child.iter_leaf_children_with_type("Data").count() != 1 {
                            // TODO: proper error message
                            panic!();
                        }
                        child.iter_leaf_children_with_type("Data").nth(0).unwrap().1
                    };

                    // Get xforms
                    let mut xforms = Vec::new();
                    for (_, contents) in child.iter_leaf_children_with_type("Transform") {
                        xforms.push(try!(parse_matrix(contents)));
                    }

                    // Add instance
                    if builder.name_exists(name) {
                        builder.add_instance(name, Some(&xforms));
                    } else {
                        // TODO: proper error message
                        panic!("Attempted to add instance for data with a name that doesn't \
                                exist.");
                    }
                }

                // MeshSurface
                "MeshSurface" => {
                    if let &DataTree::Internal {ident: Some(ident), ..} = child {
                        builder.add_object(ident, Object::Surface(Box::new(try!(parse_mesh_surface(&child)))));
                    } else {
                        // TODO: error condition of some kind, because no ident
                        panic!();
                    }
                }

                _ => {
                    // TODO: some kind of error, because not a known type name
                }

                // // Bilinear Patch
                // "BilinearPatch" => {
                //     assembly->add_object(child.name, parse_bilinear_patch(child));
                // }
                //
                // // Bicubic Patch
                // else if (child.type == "BicubicPatch") {
                //     assembly->add_object(child.name, parse_bicubic_patch(child));
                // }
                //
                // // Subdivision surface
                // else if (child.type == "SubdivisionSurface") {
                //     assembly->add_object(child.name, parse_subdivision_surface(child));
                // }
                //
                // // Sphere
                // else if (child.type == "Sphere") {
                //     assembly->add_object(child.name, parse_sphere(child));
                // }
                //
                // // Surface shader
                // else if (child.type == "SurfaceShader") {
                //     assembly->add_surface_shader(child.name, parse_surface_shader(child));
                // }
                //
                // // Sphere Light
                // else if (child.type == "SphereLight") {
                //     assembly->add_object(child.name, parse_sphere_light(child));
                // }
                //
                // // Rectangle Light
                // else if (child.type == "RectangleLight") {
                //     assembly->add_object(child.name, parse_rectangle_light(child));
                // }
                //
            }
        }
    } else {
        return Err(PsyParseError::UnknownError);
    }

    return Ok(builder.build());
}
