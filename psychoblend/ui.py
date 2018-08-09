import bpy

# Use some of the existing buttons.
from bl_ui import properties_render
properties_render.RENDER_PT_render.COMPAT_ENGINES.add('PSYCHOPATH_RENDER')
properties_render.RENDER_PT_dimensions.COMPAT_ENGINES.add('PSYCHOPATH_RENDER')
properties_render.RENDER_PT_output.COMPAT_ENGINES.add('PSYCHOPATH_RENDER')
del properties_render

from bl_ui import properties_data_camera
properties_data_camera.DATA_PT_lens.COMPAT_ENGINES.add('PSYCHOPATH_RENDER')
properties_data_camera.DATA_PT_camera.COMPAT_ENGINES.add('PSYCHOPATH_RENDER')
properties_data_camera.DATA_PT_camera_display.COMPAT_ENGINES.add('PSYCHOPATH_RENDER')
properties_data_camera.DATA_PT_custom_props_camera.COMPAT_ENGINES.add('PSYCHOPATH_RENDER')
del properties_data_camera

class PsychopathPanel():
    COMPAT_ENGINES = {'PSYCHOPATH_RENDER'}

    @classmethod
    def poll(cls, context):
        rd = context.scene.render
        return (rd.use_game_engine is False) and (rd.engine in cls.COMPAT_ENGINES)


class RENDER_PT_psychopath_render_settings(PsychopathPanel, bpy.types.Panel):
    bl_label = "Render Settings"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "render"

    def draw(self, context):
        scene = context.scene
        layout = self.layout

        col = layout.column()

        col.label(text="Sampling")
        col.prop(scene.psychopath, "spp")

        col.label(text="Dicing")
        col.prop(scene.psychopath, "dicing_rate")

        col.label(text="Motion Blur")
        col.prop(scene.psychopath, "motion_blur_segments")
        col.prop(scene.psychopath, "shutter_start")
        col.prop(scene.psychopath, "shutter_end")

        col.label(text="Performance")
        col.prop(scene.psychopath, "max_samples_per_bucket")


class RENDER_PT_psychopath_export_settings(PsychopathPanel, bpy.types.Panel):
    bl_label = "Export Settings"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "render"

    def draw(self, context):
        scene = context.scene
        layout = self.layout

        col = layout.column()
        col.prop(scene.psychopath, "export_path")


class WORLD_PT_psychopath_background(PsychopathPanel, bpy.types.Panel):
    bl_label = "Background"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "world"

    @classmethod
    def poll(cls, context):
        return context.world and PsychopathPanel.poll(context)

    def draw(self, context):
        layout = self.layout

        world = context.world
        layout.prop(world, "horizon_color", text="Color")


class DATA_PT_psychopath_camera_dof(PsychopathPanel, bpy.types.Panel):
    bl_label = "Depth of Field"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "data"

    @classmethod
    def poll(cls, context):
        engine = context.scene.render.engine
        return context.camera and PsychopathPanel.poll(context)

    def draw(self, context):
        ob = context.active_object
        layout = self.layout

        col = layout.column()

        col.prop(ob.data, "dof_object")
        col.prop(ob.data, "dof_distance")
        col.prop(ob.data.psychopath, "aperture_radius")


class DATA_PT_psychopath_lamp(PsychopathPanel, bpy.types.Panel):
    bl_label = "Lamp"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "data"

    @classmethod
    def poll(cls, context):
        engine = context.scene.render.engine
        return context.lamp and PsychopathPanel.poll(context)

    def draw(self, context):
        ob = context.active_object
        layout = self.layout

        col = layout.column()

        row = col.row()
        row.prop(ob.data, "type", expand=True)

        if ob.data.type != 'HEMI' and ob.data.type != 'AREA':
            col.prop(ob.data, "shadow_soft_size")
        col.prop(ob.data, "color")
        col.prop(ob.data, "energy")


class DATA_PT_psychopath_area_lamp(PsychopathPanel, bpy.types.Panel):
    bl_label = "Area Shape"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "data"

    @classmethod
    def poll(cls, context):
        lamp = context.lamp
        engine = context.scene.render.engine
        return (lamp and lamp.type == 'AREA') and (engine in cls.COMPAT_ENGINES)

    def draw(self, context):
        layout = self.layout

        lamp = context.lamp

        col = layout.column()
        col.row().prop(lamp, "shape", expand=True)
        sub = col.row(align=True)

        if lamp.shape == 'SQUARE':
            sub.prop(lamp, "size")
        elif lamp.shape == 'RECTANGLE':
            sub.prop(lamp, "size", text="Size X")
            sub.prop(lamp, "size_y", text="Size Y")


class DATA_PT_psychopath_mesh(PsychopathPanel, bpy.types.Panel):
    bl_label = "Psychopath Mesh Properties"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "data"

    @classmethod
    def poll(cls, context):
        engine = context.scene.render.engine
        return context.mesh and (engine in cls.COMPAT_ENGINES)

    def draw(self, context):
        layout = self.layout

        mesh = context.mesh

        layout.row().prop(mesh.psychopath, "is_subdivision_surface")


class MATERIAL_PT_psychopath_context_material(PsychopathPanel, bpy.types.Panel):
    bl_label = ""
    bl_space_type = "PROPERTIES"
    bl_region_type = "WINDOW"
    bl_context = "material"
    bl_options = {'HIDE_HEADER'}

    @classmethod
    def poll(cls, context):
        return (context.material or context.object) and PsychopathPanel.poll(context)

    def draw(self, context):
        layout = self.layout

        mat = context.material
        ob = context.object
        slot = context.material_slot
        space = context.space_data

        if ob:
            row = layout.row()

            row.template_list("MATERIAL_UL_matslots", "", ob, "material_slots", ob, "active_material_index", rows=1)

            col = row.column(align=True)
            col.operator("object.material_slot_add", icon='ZOOMIN', text="")
            col.operator("object.material_slot_remove", icon='ZOOMOUT', text="")

            col.menu("MATERIAL_MT_specials", icon='DOWNARROW_HLT', text="")

            if ob.mode == 'EDIT':
                row = layout.row(align=True)
                row.operator("object.material_slot_assign", text="Assign")
                row.operator("object.material_slot_select", text="Select")
                row.operator("object.material_slot_deselect", text="Deselect")

        split = layout.split(percentage=0.65)

        if ob:
            split.template_ID(ob, "active_material", new="material.new")
            row = split.row()

            if slot:
                row.prop(slot, "link", text="")
            else:
                row.label()
        elif mat:
            split.template_ID(space, "pin_id")
            split.separator()


class MATERIAL_PT_psychopath_surface(PsychopathPanel, bpy.types.Panel):
    bl_label = "Surface"
    bl_space_type = "PROPERTIES"
    bl_region_type = "WINDOW"
    bl_context = "material"

    @classmethod
    def poll(cls, context):
        return context.material and PsychopathPanel.poll(context)

    def draw(self, context):
        layout = self.layout

        mat = context.material
        layout.prop(mat.psychopath, "surface_shader_type")
        layout.prop(mat.psychopath, "color")

        if mat.psychopath.surface_shader_type == 'GTR':
            layout.prop(mat.psychopath, "roughness")
            layout.prop(mat.psychopath, "tail_shape")
            layout.prop(mat.psychopath, "fresnel")

        if mat.psychopath.surface_shader_type == 'GGX':
            layout.prop(mat.psychopath, "roughness")
            layout.prop(mat.psychopath, "fresnel")


def register():
    bpy.utils.register_class(RENDER_PT_psychopath_render_settings)
    bpy.utils.register_class(RENDER_PT_psychopath_export_settings)
    bpy.utils.register_class(WORLD_PT_psychopath_background)
    bpy.utils.register_class(DATA_PT_psychopath_camera_dof)
    bpy.utils.register_class(DATA_PT_psychopath_mesh)
    bpy.utils.register_class(DATA_PT_psychopath_lamp)
    bpy.utils.register_class(DATA_PT_psychopath_area_lamp)
    bpy.utils.register_class(MATERIAL_PT_psychopath_context_material)
    bpy.utils.register_class(MATERIAL_PT_psychopath_surface)

def unregister():
    bpy.utils.unregister_class(RENDER_PT_psychopath_render_settings)
    bpy.utils.unregister_class(RENDER_PT_psychopath_export_settings)
    bpy.utils.unregister_class(WORLD_PT_psychopath_background)
    bpy.utils.unregister_class(DATA_PT_psychopath_camera_dof)
    bpy.utils.register_class(DATA_PT_psychopath_mesh)
    bpy.utils.unregister_class(DATA_PT_psychopath_lamp)
    bpy.utils.unregister_class(DATA_PT_psychopath_area_lamp)
    bpy.utils.unregister_class(MATERIAL_PT_psychopath_context_material)
    bpy.utils.unregister_class(MATERIAL_PT_psychopath_surface)
