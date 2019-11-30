import bpy

from math import degrees, tan, atan
from mathutils import Vector, Matrix

from .util import escape_name, color2str, mat2str, ExportCancelled

class World:
    def __init__(self, render_engine, scene, visible_layers, aspect_ratio):
        self.background_shader = BackgroundShader(render_engine, scene.world)
        self.camera = Camera(render_engine, scene.camera, aspect_ratio)
        self.lights = []

        # Collect infinite-extent light sources.
        # TODO: also get sun lamps inside group instances.
        for ob in scene.objects:
            if ob.type == 'LAMP' and ob.data.type == 'SUN':
                name = escape_name(ob.name)
                self.lights += [DistantDiskLamp(ob, name)]

    def take_sample(self, render_engine, scene, time):
        self.camera.take_sample(render_engine, scene, time)

        for light in self.lights:
            # Check if render is cancelled
            if render_engine.test_break():
                raise ExportCancelled()
            light.take_sample(render_engine, scene, time)

    def export(self, render_engine, w):
        self.camera.export(render_engine, w)

        w.write("World {\n")
        w.indent()

        self.background_shader.export(render_engine, w)

        for light in self.lights:
            light.export(render_engine, w)

        w.unindent()
        w.write("}\n")

    def cleanup(self):
        # For future use.  This is run by the calling code when finished,
        # even if export did not succeed.
        pass

#================================================================

class Camera:
    def __init__(self, render_engine, ob, aspect_ratio):
        self.ob = ob
        self.aspect_ratio = aspect_ratio

        self.fovs = []
        self.aperture_radii = []
        self.focal_distances = []
        self.xforms = []

    def take_sample(self, render_engine, scene, time):
        render_engine.update_stats("", "Psychopath: Collecting '{}' at time {}".format(self.ob.name, time))

        # Fov
        if self.aspect_ratio >= 1.0:
            self.fovs += [degrees(self.ob.data.angle)]
        else:
            self.fovs += [degrees(2.0 * atan(tan(self.ob.data.angle * 0.5) * self.aspect_ratio))]

        # Aperture radius
        self.aperture_radii += [self.ob.data.psychopath.aperture_radius]

        # Dof distance
        if self.ob.data.dof_object == None:
            self.focal_distances += [self.ob.data.dof_distance]
        else:
            # TODO: implement DoF object tracking here
            self.focal_distances += [0.0]
            print("WARNING: DoF object tracking not yet implemented.")

        # Transform
        mat = self.ob.matrix_world.copy()
        matz = Matrix()
        matz[2][2] = -1
        self.xforms += [mat * matz]

    def export(self, render_engine, w):
        render_engine.update_stats("", "Psychopath: Exporting %s" % self.ob.name)
        w.write("Camera {\n")
        w.indent()

        for fov in self.fovs:
            w.write("Fov [{:.6}]\n".format(fov))

        for rad in self.aperture_radii:
            w.write("ApertureRadius [{:.6}]\n".format(rad))

        for dist in self.focal_distances:
            w.write("FocalDistance [{:.6}]\n".format(dist))

        for mat in self.xforms:
            w.write("Transform [{}]\n".format(mat2str(mat)))

        w.unindent()
        w.write("}\n")


class BackgroundShader:
    def __init__(self, render_engine, world):
        self.world = world
        if self.world != None:
            self.color = (world.horizon_color[0], world.horizon_color[1], world.horizon_color[2])

    def export(self, render_engine, w):
        if self.world != None:
            w.write("BackgroundShader {\n")
            w.indent();
            w.write("Type [Color]\n")
            w.write("Color [rec709, {:.6} {:.6} {:.6}]\n".format(self.color[0], self.color[1], self.color[2]))
            w.unindent()
            w.write("}\n")


class DistantDiskLamp:
    def __init__(self, ob, name):
        self.ob = ob
        self.name = name
        self.time_col = []
        self.time_dir = []
        self.time_rad = []

    def take_sample(self, render_engine, scene, time):
        render_engine.update_stats("", "Psychopath: Collecting '{}' at time {}".format(self.ob.name, time))
        self.time_dir += [tuple(self.ob.matrix_world.to_3x3() * Vector((0, 0, -1)))]

        if self.ob.data.psychopath.color_type == 'Rec709':
            self.time_col += [('Rec709', self.ob.data.color * self.ob.data.energy)]
        elif self.ob.data.psychopath.color_type == 'Blackbody':
            self.time_col += [('Blackbody', self.ob.data.psychopath.color_blackbody_temp, self.ob.data.energy)]
        elif self.ob.data.psychopath.color_type == 'ColorTemperature':
            self.time_col += [('ColorTemperature', self.ob.data.psychopath.color_blackbody_temp, self.ob.data.energy)]

        self.time_rad += [self.ob.data.shadow_soft_size]

    def export(self, render_engine, w):
        render_engine.update_stats("", "Psychopath: Exporting %s" % self.ob.name)
        w.write("DistantDiskLight $%s {\n" % self.name)
        w.indent()
        for direc in self.time_dir:
            w.write("Direction [{:.6} {:.6} {:.6}]\n".format(direc[0], direc[1], direc[2]))
        for col in self.time_col:
            w.write(color2str(col[0], col[1]) + "\n")
        for rad in self.time_rad:
            w.write("Radius [{:.6}]\n".format(rad))

        w.unindent()
        w.write("}\n")
