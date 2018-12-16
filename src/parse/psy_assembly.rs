#![allow(dead_code)]

use std::result::Result;

use mem_arena::MemArena;

use crate::scene::{Assembly, AssemblyBuilder, Object};

use super::psy::{parse_matrix, PsyParseError};
use super::psy_light::{parse_rectangle_light, parse_sphere_light};
use super::psy_mesh_surface::parse_mesh_surface;
use super::psy_surface_shader::parse_surface_shader;
use super::DataTree;

pub fn parse_assembly<'a>(
    arena: &'a MemArena,
    tree: &'a DataTree,
) -> Result<Assembly<'a>, PsyParseError> {
    let mut builder = AssemblyBuilder::new(arena);

    if tree.is_internal() {
        for child in tree.iter_children() {
            match child.type_name() {
                // Sub-Assembly
                "Assembly" => {
                    if let DataTree::Internal {
                        ident: Some(ident), ..
                    } = *child
                    {
                        builder.add_assembly(ident, parse_assembly(arena, child)?);
                    } else {
                        return Err(PsyParseError::UnknownError(child.byte_offset()));
                    }
                }

                // Instance
                "Instance" => {
                    // Pre-conditions
                    if !child.is_internal() {
                        return Err(PsyParseError::UnknownError(child.byte_offset()));
                    }

                    // Get data name
                    let name = {
                        if child.iter_leaf_children_with_type("Data").count() != 1 {
                            return Err(PsyParseError::UnknownError(child.byte_offset()));
                        }
                        child.iter_leaf_children_with_type("Data").nth(0).unwrap().1
                    };

                    // Get surface shader binding, if any.
                    let surface_shader_name = if child
                        .iter_leaf_children_with_type("SurfaceShaderBind")
                        .count()
                        > 0
                    {
                        Some(
                            child
                                .iter_leaf_children_with_type("SurfaceShaderBind")
                                .nth(0)
                                .unwrap()
                                .1,
                        )
                    } else {
                        None
                    };

                    // Get xforms
                    let mut xforms = Vec::new();
                    for (_, contents, _) in child.iter_leaf_children_with_type("Transform") {
                        xforms.push(parse_matrix(contents)?);
                    }

                    // Add instance
                    if builder.name_exists(name) {
                        builder.add_instance(name, surface_shader_name, Some(&xforms));
                    } else {
                        return Err(PsyParseError::InstancedMissingData(
                            child.iter_leaf_children_with_type("Data").nth(0).unwrap().2,
                            "Attempted to add \
                             instance for data with \
                             a name that doesn't \
                             exist.",
                            name.to_string(),
                        ));
                    }
                }

                // SurfaceShader
                "SurfaceShader" => {
                    if let DataTree::Internal {
                        ident: Some(ident), ..
                    } = *child
                    {
                        builder.add_surface_shader(ident, parse_surface_shader(arena, child)?);
                    } else {
                        // TODO: error condition of some kind, because no ident
                        panic!(
                            "SurfaceShader encountered that was a leaf, but SurfaceShaders cannot \
                             be a leaf: {}",
                            child.byte_offset()
                        );
                    }
                }

                // MeshSurface
                "MeshSurface" => {
                    if let DataTree::Internal {
                        ident: Some(ident), ..
                    } = *child
                    {
                        builder.add_object(
                            ident,
                            Object::Surface(arena.alloc(parse_mesh_surface(arena, child)?)),
                        );
                    } else {
                        // TODO: error condition of some kind, because no ident
                        panic!(
                            "MeshSurface encountered that was a leaf, but MeshSurfaces cannot \
                             be a leaf: {}",
                            child.byte_offset()
                        );
                    }
                }

                // Sphere Light
                "SphereLight" => {
                    if let DataTree::Internal {
                        ident: Some(ident), ..
                    } = *child
                    {
                        builder.add_object(
                            ident,
                            Object::SurfaceLight(arena.alloc(parse_sphere_light(arena, child)?)),
                        );
                    } else {
                        // No ident
                        return Err(PsyParseError::UnknownError(child.byte_offset()));
                    }
                }

                // Rectangle Light
                "RectangleLight" => {
                    if let DataTree::Internal {
                        ident: Some(ident), ..
                    } = *child
                    {
                        builder.add_object(
                            ident,
                            Object::SurfaceLight(arena.alloc(parse_rectangle_light(arena, child)?)),
                        );
                    } else {
                        // No ident
                        return Err(PsyParseError::UnknownError(child.byte_offset()));
                    }
                }

                _ => {
                    // TODO: some kind of error, because not a known type name
                } // // Bilinear Patch
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
            }
        }
    } else {
        return Err(PsyParseError::UnknownError(tree.byte_offset()));
    }

    return Ok(builder.build());
}
