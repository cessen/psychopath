bl_info = {
    "name": "PsychoBlend",
    "version": (0, 1),
    "author": "Nathan Vegdahl",
    "blender": (2, 70, 0),
    "description": "Psychopath renderer integration",
    "location": "",
    "wiki_url": "https://github.com/cessen/psychopath/wiki",
    "tracker_url": "https://github.com/cessen/psychopath/issues",
    "category": "Render"}


if "bpy" in locals():
    import imp
    imp.reload(ui)
    imp.reload(psy_export)
    imp.reload(render)
else:
    from . import ui, psy_export, render

import bpy
from bpy.types import (AddonPreferences,
                       PropertyGroup,
                       Operator,
                       )
from bpy.props import (StringProperty,
                       BoolProperty,
                       IntProperty,
                       FloatProperty,
                       FloatVectorProperty,
                       EnumProperty,
                       PointerProperty,
                       )


# Custom Scene settings
class RenderPsychopathSettingsScene(PropertyGroup):
    spp = IntProperty(
        name="Samples Per Pixel", description="Total number of samples to take per pixel",
        min=1, max=65536, default=16
        )

    max_samples_per_bucket = IntProperty(
        name="Max Samples Per Bucket", description="How many samples to simultaneously calculate per thread; indirectly determines bucket size",
        min=1, max=2**28, soft_max=2**16, default=4096
        )

    dicing_rate = FloatProperty(
        name="Dicing Rate", description="The target microgeometry width in pixels",
        min=0.0001, max=100.0, soft_min=0.125, soft_max=1.0, default=0.25
        )

    motion_blur_segments = IntProperty(
        name="Motion Segments", description="The number of segments to use in motion blur.  Zero means no motion blur.  Will be rounded down to the nearest power of two.",
        min=0, max=256, default=0
        )

    shutter_start = FloatProperty(
        name="Shutter Open", description="The time during the frame that the shutter opens, for motion blur",
        min=-1.0, max=1.0, soft_min=0.0, soft_max=1.0, default=0.0
        )

    shutter_end = FloatProperty(
        name="Shutter Close", description="The time during the frame that the shutter closes, for motion blur",
        min=-1.0, max=1.0, soft_min=0.0, soft_max=1.0, default=0.5
        )

    export_path = StringProperty(
        name="Export Path", description="The path to where the .psy files should be exported when rendering.  If left blank, /tmp or the equivalent is used.",
        subtype='FILE_PATH'
        )

# Custom Camera properties
class PsychopathCamera(bpy.types.PropertyGroup):
    aperture_radius = FloatProperty(
        name="Aperture Radius", description="Size of the camera's aperture, for DoF",
        min=0.0, max=10000.0, soft_min=0.0, soft_max=2.0, default=0.0
        )

# Custom Mesh properties
class PsychopathMesh(bpy.types.PropertyGroup):
    is_subdivision_surface = BoolProperty(
        name="Is Subdivision Surface", description="Whether this is a sibdivision surface or just a normal mesh",
        default=False
        )

# Psychopath material
class PsychopathMaterial(bpy.types.PropertyGroup):
    surface_shader_type = EnumProperty(
        name="Surface Shader Type", description="",
        items=[('Emit', 'Emit', ""), ('Lambert', 'Lambert', ""), ('GTR', 'GTR', ""), ('GGX', 'GGX', "")],
        default="Lambert"
        )

    color = FloatVectorProperty(
        name="Color", description="",
        subtype='COLOR',
        min=0.0, soft_min=0.0, soft_max = 1.0,
        default=[0.8,0.8,0.8]
        )

    roughness = FloatProperty(
        name="Roughness", description="",
        min=-1.0, max=1.0, soft_min=0.0, soft_max=1.0, default=0.1
        )

    tail_shape = FloatProperty(
        name="Tail Shape", description="",
        min=0.0, max=8.0, soft_min=1.0, soft_max=3.0, default=2.0
        )

    fresnel = FloatProperty(
        name="Fresnel", description="",
        min=0.0, max=1.0, soft_min=0.0, soft_max=1.0, default=0.9
        )


# Addon Preferences
class PsychopathPreferences(AddonPreferences):
    bl_idname = __name__

    filepath_psychopath = StringProperty(
                name="Psychopath Location",
                description="Path to renderer executable",
                subtype='DIR_PATH',
                )

    def draw(self, context):
        layout = self.layout
        layout.prop(self, "filepath_psychopath")


##### REGISTER #####
def register():
    bpy.utils.register_class(PsychopathPreferences)
    bpy.utils.register_class(RenderPsychopathSettingsScene)
    bpy.utils.register_class(PsychopathCamera)
    bpy.utils.register_class(PsychopathMesh)
    bpy.utils.register_class(PsychopathMaterial)
    bpy.types.Scene.psychopath = PointerProperty(type=RenderPsychopathSettingsScene)
    bpy.types.Camera.psychopath = PointerProperty(type=PsychopathCamera)
    bpy.types.Mesh.psychopath = PointerProperty(type=PsychopathMesh)
    bpy.types.Material.psychopath = PointerProperty(type=PsychopathMaterial)
    render.register()
    ui.register()


def unregister():
    bpy.utils.unregister_class(PsychopathPreferences)
    bpy.utils.unregister_class(RenderPsychopathSettingsScene)
    bpy.utils.unregister_class(PsychopathCamera)
    bpy.utils.unregister_class(PsychopathMesh)
    bpy.utils.unregister_class(PsychopathMaterial)
    del bpy.types.Scene.psychopath
    del bpy.types.Camera.psychopath
    del bpy.types.Mesh.psychopath
    del bpy.types.Material.psychopath
    render.unregister()
    ui.unregister()
