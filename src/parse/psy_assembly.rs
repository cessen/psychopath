#![allow(dead_code)]

use std::result::Result;

use nom;
use nom::IResult;

use super::DataTree;
use super::basics::{ws_u32, ws_f32};
use super::psy::PsyParseError;

use math::Matrix4x4;
use camera::Camera;
use renderer::Renderer;
use assembly::{Assembly, AssemblyBuilder};

pub fn parse_assembly(tree: &DataTree) -> Result<Assembly, PsyParseError> {
    let mut builder = AssemblyBuilder::new();

    if let &DataTree::Internal{ref children, ..} = tree {
        for child in children {
            match child.type_name() {
                // Sub-Assembly
                "Assembly" => {
                    if let &DataTree::Internal {ident: Some(ident), ..} = child {
                        builder.add_assembly(ident, try!(parse_assembly(&child)));
                    } else {
                        // TODO: error condition of some kind, because no ident
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
                // // Instance
                // else if (child.type == "Instance") {
                //     // Parse
                //     std::string name = "";
                //     std::vector<Transform> xforms;
                //     const SurfaceShader *shader = nullptr;
                //     for (const auto& child2: child.children) {
                //         if (child2.type == "Transform") {
                //             xforms.emplace_back(parse_matrix(child2.leaf_contents));
                //         } else if (child2.type == "Data") {
                //             name = child2.leaf_contents;
                //         } else if (child2.type == "SurfaceShaderBind") {
                //             shader = assembly->get_surface_shader(child2.leaf_contents);
                //             if (shader == nullptr) {
                //                 std::cout << "ERROR: attempted to bind surface shader that doesn't exist." << std::endl;
                //             }
                //         }
                //     }
                //
                //     // Add instance
                //     if (assembly->object_map.count(name) != 0) {
                //         assembly->create_object_instance(name, xforms, shader);
                //     } else if (assembly->assembly_map.count(name) != 0) {
                //         assembly->create_assembly_instance(name, xforms, shader);
                //     } else {
                //         std::cout << "ERROR: attempted to add instace for data that doesn't exist." << std::endl;
                //     }
                // }
            }
        }
    } else {
        return Err(PsyParseError::UnknownError);
    }

    return Ok(builder.build());
}
