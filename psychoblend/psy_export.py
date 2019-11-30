import bpy

from math import log

from .assembly import Assembly
from .util import escape_name, mat2str, ExportCancelled
from .world import World


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
        self.w.write("DicingRate [{:.6}]\n".format(self.scene.psychopath.dicing_rate))
        self.w.write('Seed [%d]\n' % self.fr)

        # RenderSettings section end
        self.w.unindent()
        self.w.write("}\n")

        ###############################
        # Export world and object data
        world = None
        root_assembly = None
        try:
            # Prep for data collection
            world = World(self.render_engine, self.scene, self.scene.layers, float(res_x) / float(res_y))
            root_assembly = Assembly(self.render_engine, self.scene.objects, self.scene.layers)

            # Collect data for each time sample
            for i in range(self.time_samples):
                time = self.fr + self.shutter_start + (self.shutter_diff*i)
                self.set_frame(self.fr, self.shutter_start + (self.shutter_diff*i))
                world.take_sample(self.render_engine, self.scene, time)
                root_assembly.take_sample(self.render_engine, self.scene, time)

            # Export collected data
            world.export(self.render_engine, self.w)
            root_assembly.export(self.render_engine, self.w)
        finally:
            if world != None:
                world.cleanup()
            if root_assembly != None:
                root_assembly.cleanup()

        # Scene end
        self.w.unindent()
        self.w.write("}\n")
