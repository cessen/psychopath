import bpy

from .util import escape_name, mat2str, color2str, psycolor2str, needs_def_mb, needs_xform_mb, ExportCancelled

class Assembly:
    def __init__(self, render_engine, objects, visible_layers, group_prefix="", translation_offset=(0,0,0)):
        self.name = group_prefix
        self.translation_offset = translation_offset
        self.render_engine = render_engine
        
        self.materials = []
        self.objects = []
        self.instances = []

        self.material_names = set()
        self.mesh_names = set()
        self.assembly_names = set()

        # Collect all the objects, materials, instances, etc.
        for ob in objects:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()

            # Check if the object is visible for rendering
            vis_layer = False
            for i in range(len(ob.layers)):
                vis_layer = vis_layer or (ob.layers[i] and visible_layers[i])
            if ob.hide_render or not vis_layer:
                continue

            # Store object data
            name = None

            if ob.type == 'EMPTY':
                if ob.dupli_type == 'GROUP':
                    name = group_prefix + "__" + escape_name(ob.dupli_group.name)
                    if name not in self.assembly_names:
                        self.assembly_names.add(name)
                        self.objects += [Assembly(self.render_engine, ob.dupli_group.objects, ob.dupli_group.layers, name, ob.dupli_group.dupli_offset*-1)]
            elif ob.type == 'MESH':
                name = self.get_mesh(ob, group_prefix)
            elif ob.type == 'LAMP' and ob.data.type == 'POINT':
                name = self.get_sphere_lamp(ob, group_prefix)
            elif ob.type == 'LAMP' and ob.data.type == 'AREA':
                name = self.get_rect_lamp(ob, group_prefix)
            
            # Store instance
            if name != None:
                self.instances += [Instance(render_engine, ob, name)]

    def export(self, render_engine, w):
        if self.name == "":
            w.write("Assembly {\n")
        else:
            w.write("Assembly $%s {\n" % self.name)
        w.indent()

        for mat in self.materials:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()
            mat.export(render_engine, w)

        for ob in self.objects:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()
            ob.export(render_engine, w)

        for inst in self.instances:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()
            inst.export(render_engine, w)

        w.unindent()
        w.write("}\n")
    
    #----------------

    def take_sample(self, render_engine, scene, time):
        for mat in self.materials:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()
            mat.take_sample(render_engine, scene, time)

        for ob in self.objects:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()
            ob.take_sample(render_engine, scene, time)

        for inst in self.instances:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()
            inst.take_sample(render_engine, time, self.translation_offset)
    
    def cleanup(self):
        for mat in self.materials:
            mat.cleanup()
        for ob in self.objects:
            ob.cleanup()

    def get_mesh(self, ob, group_prefix):
        # Figure out if we need to export or not and figure out what name to
        # export with.
        has_modifiers = len(ob.modifiers) > 0
        deform_mb = needs_def_mb(ob)
        if has_modifiers or deform_mb:
            mesh_name = group_prefix + escape_name("__" + ob.name + "__" + ob.data.name + "_")
        else:
            mesh_name = group_prefix + escape_name("__" + ob.data.name + "_")
        has_faces = len(ob.data.polygons) > 0
        should_export_mesh = has_faces and (mesh_name not in self.mesh_names)
        
        # Get mesh
        if should_export_mesh:
            self.mesh_names.add(mesh_name)
            self.objects += [Mesh(self.render_engine, ob, mesh_name)]

            # Get materials
            for ms in ob.material_slots:
                if ms != None:
                    if ms.material.name not in self.material_names:
                        self.material_names.add(ms.material.name)
                        self.materials += [Material(self.render_engine, ms.material)]

            return mesh_name
        else:
            return None


    def get_sphere_lamp(self, ob, group_prefix):
        name = group_prefix + "__" + escape_name(ob.name)
        self.objects += [SphereLamp(self.render_engine, ob, name)]
        return name

    def get_rect_lamp(self, ob, group_prefix):
        name = group_prefix + "__" + escape_name(ob.name)
        self.objects += [RectLamp(self.render_engine, ob, name)]
        return name


#=========================================================================


class Mesh:
    """ Holds data for a mesh to be exported.
    """
    def __init__(self, render_engine, ob, name):
        self.ob = ob
        self.name = name
        self.needs_mb = needs_def_mb(self.ob)
        self.time_meshes = []

    def take_sample(self, render_engine, scene, time):
        if len(self.time_meshes) == 0 or self.needs_mb:
            render_engine.update_stats("", "Psychopath: Collecting '{}' at time {}".format(self.ob.name, time))
            self.time_meshes += [self.ob.to_mesh(scene, True, 'RENDER')]
    
    def cleanup(self):
        for mesh in self.time_meshes:
            bpy.data.meshes.remove(mesh)

    def export(self, render_engine, w):
        render_engine.update_stats("", "Psychopath: Exporting %s" % self.ob.name)

        if self.ob.data.psychopath.is_subdivision_surface == False:
            # Exporting normal mesh
            w.write("MeshSurface $%s {\n" % self.name)
            w.indent()
        else:
            # Exporting subdivision surface cage
            w.write("SubdivisionSurface $%s {\n" % self.name)
            w.indent()

        # Write vertices and (if it's smooth shaded) normals
        for ti in range(len(self.time_meshes)):
            w.write("Vertices [")
            w.write(" ".join(["{:.6} {:.6} {:.6}".format(vert.co[0], vert.co[1], vert.co[2]) for vert in self.time_meshes[ti].vertices]), False)
            w.write("]\n", False)
            if self.time_meshes[0].polygons[0].use_smooth and self.ob.data.psychopath.is_subdivision_surface == False:
                w.write("Normals [")
                w.write(" ".join(["{:.6} {:.6} {:.6}".format(vert.normal[0], vert.normal[1], vert.normal[2]) for vert in self.time_meshes[ti].vertices]), False)
                w.write("]\n", False)

        # Write face vertex counts
        w.write("FaceVertCounts [")
        w.write(" ".join(["{}".format(len(p.vertices)) for p in self.time_meshes[0].polygons]), False)
        w.write("]\n", False)

        # Write face vertex indices
        w.write("FaceVertIndices [")
        w.write(" ".join(["{}".format(v) for p in self.time_meshes[0].polygons for v in p.vertices]), False)
        w.write("]\n", False)

        # MeshSurface/SubdivisionSurface section end
        w.unindent()
        w.write("}\n")


class SphereLamp:
    """ Holds data for a sphere light to be exported.
    """
    def __init__(self, render_engine, ob, name):
        self.ob = ob
        self.name = name
        self.time_col = []
        self.time_rad = []

    def take_sample(self, render_engine, scene, time):
        render_engine.update_stats("", "Psychopath: Collecting '{}' at time {}".format(self.ob.name, time))

        if self.ob.data.psychopath.color_type == 'Rec709':
            self.time_col += [('Rec709', self.ob.data.color * self.ob.data.energy)]
        elif self.ob.data.psychopath.color_type == 'Blackbody':
            self.time_col += [('Blackbody', self.ob.data.psychopath.color_blackbody_temp, self.ob.data.energy)]
        elif self.ob.data.psychopath.color_type == 'ColorTemperature':
            self.time_col += [('ColorTemperature', self.ob.data.psychopath.color_blackbody_temp, self.ob.data.energy)]

        self.time_rad += [self.ob.data.shadow_soft_size]

    def cleanup(self):
        pass
    
    def export(self, render_engine, w):
        render_engine.update_stats("", "Psychopath: Exporting %s" % self.ob.name)

        w.write("SphereLight $%s {\n" % self.name)
        w.indent()
        for col in self.time_col:
            w.write(color2str(col[0], col[1]) + "\n")
        for rad in self.time_rad:
            w.write("Radius [{:.6}]\n".format(rad))

        w.unindent()
        w.write("}\n")


class RectLamp:
    """ Holds data for a rectangular light to be exported.
    """
    def __init__(self, render_engine, ob, name):
        self.ob = ob
        self.name = name
        self.time_col = []
        self.time_dim = []

    def take_sample(self, render_engine, scene, time):
        render_engine.update_stats("", "Psychopath: Collecting '{}' at time {}".format(self.ob.name, time))

        if self.ob.data.psychopath.color_type == 'Rec709':
            self.time_col += [('Rec709', self.ob.data.color * self.ob.data.energy)]
        elif self.ob.data.psychopath.color_type == 'Blackbody':
            self.time_col += [('Blackbody', self.ob.data.psychopath.color_blackbody_temp, self.ob.data.energy)]
        elif self.ob.data.psychopath.color_type == 'ColorTemperature':
            self.time_col += [('ColorTemperature', self.ob.data.psychopath.color_blackbody_temp, self.ob.data.energy)]

        if self.ob.data.shape == 'RECTANGLE':
            self.time_dim += [(self.ob.data.size, self.ob.data.size_y)]
        else:
            self.time_dim += [(self.ob.data.size, self.ob.data.size)]
    
    def cleanup(self):
        pass
    
    def export(self, render_engine, w):
        render_engine.update_stats("", "Psychopath: Exporting %s" % self.ob.name)

        w.write("RectangleLight $%s {\n" % self.name)
        w.indent()
        for col in self.time_col:
            w.write(color2str(col[0], col[1]) + "\n")
        for dim in self.time_dim:
            w.write("Dimensions [{:.6} {:.6}]\n".format(dim[0], dim[1]))

        w.unindent()
        w.write("}\n")


class Instance:
    def __init__(self, render_engine, ob, data_name):
        self.ob = ob
        self.data_name = data_name
        self.needs_mb = needs_xform_mb(self.ob)
        self.time_xforms = []

    def take_sample(self, render_engine, time, translation_offset):
        if len(self.time_xforms) == 0 or self.needs_mb:
            render_engine.update_stats("", "Psychopath: Collecting '{}' xforms at time {}".format(self.ob.name, time))
            mat = self.ob.matrix_world.copy()
            mat[0][3] += translation_offset[0]
            mat[1][3] += translation_offset[1]
            mat[2][3] += translation_offset[2]
            self.time_xforms += [mat]

    def export(self, render_engine, w):
        render_engine.update_stats("", "Psychopath: Exporting %s" % self.ob.name)

        w.write("Instance {\n")
        w.indent()
        w.write("Data [$%s]\n" % self.data_name)
        for mat in self.time_xforms:
            w.write("Transform [%s]\n" % mat2str(mat.inverted()))
        for ms in self.ob.material_slots:
            if ms != None:
                w.write("SurfaceShaderBind [$%s]\n" % escape_name(ms.material.name))
                break
        w.unindent()
        w.write("}\n")


class Material:
    def __init__(self, render_engine, material):
        self.mat = material

    def take_sample(self, render_engine, time, translation_offset):
        # TODO: motion blur of material settings
        pass

    def export(self, render_engine, w):
        render_engine.update_stats("", "Psychopath: Exporting %s" % self.mat.name)

        w.write("SurfaceShader $%s {\n" % escape_name(self.mat.name))
        w.indent()
        if self.mat.psychopath.surface_shader_type == 'Emit':
            w.write("Type [Emit]\n")
            w.write(psycolor2str(self.mat.psychopath) + "\n")
        elif self.mat.psychopath.surface_shader_type == 'Lambert':
            w.write("Type [Lambert]\n")
            w.write(psycolor2str(self.mat.psychopath) + "\n")
        elif self.mat.psychopath.surface_shader_type == 'GGX':
            w.write("Type [GGX]\n")
            w.write(psycolor2str(self.mat.psychopath) + "\n")
            w.write("Roughness [{:.6}]\n".format(self.mat.psychopath.roughness))
            w.write("Fresnel [{:.6}]\n".format(self.mat.psychopath.fresnel))
        else:
            raise "Unsupported surface shader type '%s'" % self.mat.psychopath.surface_shader_type
        w.unindent()
        w.write("}\n")

    def cleanup(self):
        pass
