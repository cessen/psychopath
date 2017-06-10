import bpy

from math import degrees, pi, log
from mathutils import Vector, Matrix

from .assembly import Assembly
from .util import escape_name, mat2str

class ExportCancelled(Exception):
    """ Indicates that the render was cancelled in the middle of exporting
        the scene file.
    """
    pass


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
            self.f.write(bytes(' '*self.indent_level + text, "utf-8"))
        else:
            self.f.write(bytes(text, "utf-8"))


class PsychoExporter:
    def __init__(self, f, render_engine, scene):
        self.w = IndentedWriter(f)
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

    def export_psy(self):
        try:
            self._export_psy()
        except ExportCancelled:
            # Cleanup
            self.scene.frame_set(self.fr)
            return False
        else:
            # Cleanup
            self.scene.frame_set(self.fr)
            return True

    def _export_psy(self):
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
        try:
            root_assembly = Assembly(self.render_engine, self.scene.objects, self.scene.layers)
            for i in range(self.time_samples):
                time = self.fr + self.shutter_start + (self.shutter_diff*i)
                self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
                root_assembly.take_sample(self.render_engine, self.scene, time)
            root_assembly.export(self.render_engine, self.w)
        except ExportCancelled:
            root_assembly.cleanup()
            raise ExportCancelled()
        else:
            root_assembly.cleanup()

        # Scene end
        self.w.unindent()
        self.w.write("}\n")

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
