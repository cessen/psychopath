import bpy

from math import degrees, pi, log
from mathutils import Vector, Matrix

class ExportCancelled(Exception):
    """ Indicates that the render was cancelled in the middle of exporting
        the scene file.
    """
    pass


def mat2str(m):
    """ Converts a matrix into a single-line string of values.
    """
    s = ""
    for j in range(4):
        for i in range(4):
            s += (" %f" % m[i][j])
    return s[1:]


def needs_def_mb(ob):
    """ Determines if the given object needs to be exported with
        deformation motion blur or not.
    """
    for mod in ob.modifiers:
        if mod.type == 'SUBSURF':
            pass
        elif mod.type == 'MIRROR':
            if mod.mirror_object == None:
                pass
            else:
                return True
        else:
            return True

    if ob.type == 'MESH':
        if ob.data.shape_keys == None:
            pass
        else:
            return True

    return False

def escape_name(name):
    name = name.replace("\\", "\\\\")
    name = name.replace(" ", "\\ ")
    name = name.replace("$", "\\$")
    name = name.replace("[", "\\[")
    name = name.replace("]", "\\]")
    name = name.replace("{", "\\{")
    name = name.replace("}", "\\}")
    return name


def needs_xform_mb(ob):
    """ Determines if the given object needs to be exported with
        transformation motion blur or not.
    """
    if ob.animation_data != None:
        return True

    if len(ob.constraints) > 0:
        return True

    if ob.parent != None:
        return needs_xform_mb(ob.parent)

    return False


class IndentedWriter:
    def __init__(self, file_handle):
        self.f = file_handle
        self.indent_level = 0
        self.indent_size = 4

    def indent(self):
        self.indent_level += self.indent_size

    def unindent(self):
        self.indent_level -= self.indent_size
        if self.indent_level < 0:
            self.indent_level = 0

    def write(self, text, do_indent=True):
        if do_indent:
            self.f.write(' '*self.indent_level + text)
        else:
            self.f.write(text)



class PsychoExporter:
    def __init__(self, render_engine, scene):
        self.render_engine = render_engine
        self.scene = scene

        self.mesh_names = {}
        self.group_names = {}

        # Motion blur segments are rounded down to a power of two
        if scene.psychopath.motion_blur_segments > 0:
            self.time_samples = (2**int(log(scene.psychopath.motion_blur_segments, 2))) + 1
        else:
            self.time_samples = 1

        # pre-calculate useful values for exporting motion blur
        self.shutter_start = scene.psychopath.shutter_start
        self.shutter_diff = (scene.psychopath.shutter_end - scene.psychopath.shutter_start) / max(1, (self.time_samples-1))

        self.fr = scene.frame_current


    def set_frame(self, frame, fraction):
        if fraction >= 0:
            self.scene.frame_set(frame, fraction)
        else:
            self.scene.frame_set(frame-1, 1.0+fraction)

    def export_psy(self, export_path):
        try:
            f = open(export_path, 'w')
            self._export_psy(f, export_path)
        except ExportCancelled:
            # Cleanup
            f.close()
            self.scene.frame_set(self.fr)
            return False
        else:
            # Cleanup
            f.close()
            self.scene.frame_set(self.fr)
            return True

    def _export_psy(self, f, export_path):
        self.w = IndentedWriter(f)

        # Info
        self.w.write("# Exported from Blender 2.7x\n")

        # Scene begin
        self.w.write("\n\nScene $%s_fr%d {\n" % (escape_name(self.scene.name), self.fr))
        self.w.indent()

        #######################
        # Output section begin
        self.w.write("Output {\n")
        self.w.indent()

        self.w.write('Path [""]\n')

        # Output section end
        self.w.unindent()
        self.w.write("}\n")

        ###############################
        # RenderSettings section begin
        self.w.write("RenderSettings {\n")
        self.w.indent()

        res_x = int(self.scene.render.resolution_x * (self.scene.render.resolution_percentage / 100))
        res_y = int(self.scene.render.resolution_y * (self.scene.render.resolution_percentage / 100))
        self.w.write('Resolution [%d %d]\n' % (res_x, res_y))
        self.w.write("SamplesPerPixel [%d]\n" % self.scene.psychopath.spp)
        self.w.write("DicingRate [%f]\n" % self.scene.psychopath.dicing_rate)
        self.w.write('Seed [%d]\n' % self.fr)

        # RenderSettings section end
        self.w.unindent()
        self.w.write("}\n")

        #######################
        # Camera section begin
        self.w.write("Camera {\n")
        self.w.indent()

        cam = self.scene.camera

        if cam.data.dof_object == None:
            dof_distance = cam.data.dof_distance
        else:
            # TODO: implement DoF object tracking here
            dof_distance = 0.0
            print("WARNING: DoF object tracking not yet implemented.")

        matz = Matrix()
        matz[2][2] = -1
        for i in range(self.time_samples):
            # Check if render is cancelled
            if self.render_engine.test_break():
                raise ExportCancelled()

            if res_x >= res_y:
                self.w.write("Fov [%f]\n" % degrees(cam.data.angle))
            else:
                self.w.write("Fov [%f]\n" % (degrees(cam.data.angle) * res_x / res_y))
            self.w.write("FocalDistance [%f]\n" % dof_distance)
            self.w.write("ApertureRadius [%f]\n" % (cam.data.psychopath.aperture_radius))
            if self.time_samples > 1:
                self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
            mat = cam.matrix_world.copy()
            mat = mat * matz
            self.w.write("Transform [%s]\n" % mat2str(mat))

        # Camera section end
        self.w.unindent()
        self.w.write("}\n")

        #######################
        # World section begin
        self.w.write("World {\n")
        self.w.indent()

        world = self.scene.world

        if world != None:
            self.w.write("BackgroundShader {\n")
            self.w.indent();
            self.w.write("Type [Color]\n")
            self.w.write("Color [%f %f %f]\n" % (world.horizon_color[0], world.horizon_color[1], world.horizon_color[2]))
            self.w.unindent();
            self.w.write("}\n")

        # Infinite light sources
        for ob in self.scene.objects:
            if ob.type == 'LAMP' and ob.data.type == 'SUN':
                self.export_world_distant_disk_lamp(ob, "")

        # World section end
        self.w.unindent()
        self.w.write("}\n")

        #######################
        # Export objects and materials
        # TODO: handle materials from linked files (as used in group
        # instances) properly.
        self.w.write("Assembly {\n")
        self.w.indent()
        self.export_materials(bpy.data.materials)
        self.export_objects(self.scene.objects, self.scene.layers)
        self.w.unindent()
        self.w.write("}\n")

        # Scene end
        self.w.unindent()
        self.w.write("}\n")



    def export_materials(self, materials):
        for m in materials:
            self.w.write("SurfaceShader $%s {\n" % escape_name(m.name))
            self.w.indent()
            self.w.write("Type [%s]\n" % m.psychopath.surface_shader_type)
            self.w.write("Color [%f %f %f]\n" % (m.psychopath.color[0], m.psychopath.color[1], m.psychopath.color[2]))
            if m.psychopath.surface_shader_type == 'GTR':
                self.w.write("Roughness [%f]\n" % m.psychopath.roughness)
                self.w.write("TailShape [%f]\n" % m.psychopath.tail_shape)
                self.w.write("Fresnel [%f]\n" % m.psychopath.fresnel)
            self.w.unindent()
            self.w.write("}\n")


    def export_objects(self, objects, visible_layers, group_prefix="", translation_offset=(0,0,0)):
        for ob in objects:
            # Check if render is cancelled
            if self.render_engine.test_break():
                raise ExportCancelled()

            # Check if the object is visible for rendering
            vis_layer = False
            for i in range(len(ob.layers)):
                vis_layer = vis_layer or (ob.layers[i] and visible_layers[i])
            if ob.hide_render or not vis_layer:
                continue

            name = None

            # Write object data
            if ob.type == 'EMPTY':
                if ob.dupli_type == 'GROUP':
                    name = group_prefix + "__" + escape_name(ob.dupli_group.name)
                    if name not in self.group_names:
                        self.group_names[name] = True
                        self.w.write("Assembly $%s {\n" % name)
                        self.w.indent()
                        self.export_objects(ob.dupli_group.objects, ob.dupli_group.layers, name, ob.dupli_group.dupli_offset*-1)
                        self.w.unindent()
                        self.w.write("}\n")
            elif ob.type == 'MESH':
                name = self.export_mesh_object(ob, group_prefix)
            elif ob.type == 'SURFACE':
                name = self.export_surface_object(ob, group_prefix)
            elif ob.type == 'LAMP' and ob.data.type == 'POINT':
                name = self.export_sphere_lamp(ob, group_prefix)
            elif ob.type == 'LAMP' and ob.data.type == 'AREA':
                name = self.export_area_lamp(ob, group_prefix)

            # Write object instance, with transforms
            if name != None:
                time_mats = []

                if needs_xform_mb(ob):
                    for i in range(self.time_samples):
                        # Check if render is cancelled
                        if self.render_engine.test_break():
                            raise ExportCancelled()
                        self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
                        mat = ob.matrix_world.copy()
                        mat[0][3] += translation_offset[0]
                        mat[1][3] += translation_offset[1]
                        mat[2][3] += translation_offset[2]
                        time_mats += [mat]
                else:
                    mat = ob.matrix_world.copy()
                    mat[0][3] += translation_offset[0]
                    mat[1][3] += translation_offset[1]
                    mat[2][3] += translation_offset[2]
                    time_mats += [mat]

                self.w.write("Instance {\n")
                self.w.indent()
                self.w.write("Data [$%s]\n" % name)
                if len(ob.material_slots) > 0 and ob.material_slots[0].material != None:
                    self.w.write("SurfaceShaderBind [$%s]\n" % escape_name(ob.material_slots[0].material.name))
                for i in range(len(time_mats)):
                    mat = time_mats[i].inverted()
                    self.w.write("Transform [%s]\n" % mat2str(mat))
                self.w.unindent()
                self.w.write("}\n")


    def export_mesh_object(self, ob, group_prefix):
        # Determine if and how to export the mesh data
        has_modifiers = len(ob.modifiers) > 0
        deform_mb = needs_def_mb(ob)
        if has_modifiers or deform_mb:
            mesh_name = group_prefix + escape_name("__" + ob.name + "__" + ob.data.name + "_")
        else:
            mesh_name = group_prefix + escape_name("__" + ob.data.name + "_")
        export_mesh = (mesh_name not in self.mesh_names) or has_modifiers or deform_mb

        # Collect time samples
        time_meshes = []
        if deform_mb:
            for i in range(self.time_samples):
                # Check if render is cancelled
                if self.render_engine.test_break():
                    raise ExportCancelled()
                self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
                if export_mesh and (deform_mb or i == 0):
                    time_meshes += [ob.to_mesh(self.scene, True, 'RENDER')]
        elif export_mesh:
            time_meshes += [ob.to_mesh(self.scene, True, 'RENDER')]

        # Export mesh data if necessary
        if export_mesh:
            if ob.data.psychopath.is_subdivision_surface == False:
                # Exporting normal mesh
                self.mesh_names[mesh_name] = True
                self.w.write("MeshSurface $%s {\n" % mesh_name)
                self.w.indent()
            elif ob.data.psychopath.is_subdivision_surface == True:
                # Exporting subdivision surface cage
                self.mesh_names[mesh_name] = True
                self.w.write("SubdivisionSurface $%s {\n" % mesh_name)
                self.w.indent()

            # Write vertices
            for ti in range(len(time_meshes)):
                self.w.write("Vertices [")
                self.w.write(" ".join([("%f" % i) for vert in time_meshes[ti].vertices for i in vert.co]), False)
                self.w.write("]\n", False)

            # Write face vertex counts
            self.w.write("FaceVertCounts [")
            self.w.write(" ".join([("%d" % len(p.vertices)) for p in time_meshes[0].polygons]), False)
            self.w.write("]\n", False)

            # Write face vertex indices
            self.w.write("FaceVertIndices [")
            self.w.write(" ".join([("%d"%v) for p in time_meshes[0].polygons for v in p.vertices]), False)
            self.w.write("]\n", False)

            # MeshSurface/SubdivisionSurface section end
            self.w.unindent()
            self.w.write("}\n")

        for mesh in time_meshes:
            bpy.data.meshes.remove(mesh)

        return mesh_name


    def export_surface_object(self, ob, group_prefix):
        name = group_prefix + "__" + escape_name(ob.name)

        # Collect time samples
        time_surfaces = []
        for i in range(self.time_samples):
            # Check if render is cancelled
            if self.render_engine.test_break():
                raise ExportCancelled()
            self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
            time_surfaces += [ob.data.copy()]

        # Write patch
        self.w.write("BicubicPatch $" + name + " {\n")
        self.w.indent()
        for i in range(self.time_samples):
            verts = time_surfaces[i].splines[0].points
            vstr = ""
            for v in verts:
                vstr += ("%f %f %f " % (v.co[0], v.co[1], v.co[2]))
            self.w.write("Vertices [%s]\n" % vstr[:-1])
        for s in time_surfaces:
            bpy.data.curves.remove(s)
        self.w.unindent()
        self.w.write("}\n")

        return name


    def export_sphere_lamp(self, ob, group_prefix):
        name = group_prefix + "__" + escape_name(ob.name)

        # Collect data over time
        time_col = []
        time_rad = []
        for i in range(self.time_samples):
            # Check if render is cancelled
            if self.render_engine.test_break():
                raise ExportCancelled()
            self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
            time_col += [ob.data.color * ob.data.energy]
            time_rad += [ob.data.shadow_soft_size]

        # Write out sphere light
        self.w.write("SphereLight $%s {\n" % name)
        self.w.indent()
        for col in time_col:
            self.w.write("Color [%f %f %f]\n" % (col[0], col[1], col[2]))
        for rad in time_rad:
            self.w.write("Radius [%f]\n" % rad)

        self.w.unindent()
        self.w.write("}\n")

        return name

    def export_area_lamp(self, ob, group_prefix):
        name = group_prefix + "__" + escape_name(ob.name)

        # Collect data over time
        time_col = []
        time_dim = []
        for i in range(self.time_samples):
            # Check if render is cancelled
            if self.render_engine.test_break():
                raise ExportCancelled()
            self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
            time_col += [ob.data.color * ob.data.energy]
            if ob.data.shape == 'RECTANGLE':
                time_dim += [(ob.data.size, ob.data.size_y)]
            else:
                time_dim += [(ob.data.size, ob.data.size)]


        # Write out sphere light
        self.w.write("RectangleLight $%s {\n" % name)
        self.w.indent()
        for col in time_col:
            self.w.write("Color [%f %f %f]\n" % (col[0], col[1], col[2]))
        for dim in time_dim:
            self.w.write("Dimensions [%f %f]\n" % dim)

        self.w.unindent()
        self.w.write("}\n")

        return name

    def export_world_distant_disk_lamp(self, ob, group_prefix):
        name = group_prefix + "__" + escape_name(ob.name)

        # Collect data over time
        time_dir = []
        time_col = []
        time_rad = []
        for i in range(self.time_samples):
            # Check if render is cancelled
            if self.render_engine.test_break():
                raise ExportCancelled()
            self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
            time_dir += [tuple(ob.matrix_world.to_3x3() * Vector((0, 0, -1)))]
            time_col += [ob.data.color * ob.data.energy]
            time_rad += [ob.data.shadow_soft_size]

        # Write out sphere light
        self.w.write("DistantDiskLight $%s {\n" % name)
        self.w.indent()
        for direc in time_dir:
            self.w.write("Direction [%f %f %f]\n" % (direc[0], direc[1], direc[2]))
        for col in time_col:
            self.w.write("Color [%f %f %f]\n" % (col[0], col[1], col[2]))
        for rad in time_rad:
            self.w.write("Radius [%f]\n" % rad)

        self.w.unindent()
        self.w.write("}\n")

        return name
