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
    anim = ob.animation_data
    no_anim_data = anim == None or (anim.action == None and len(anim.nla_tracks) == 0 and len(anim.drivers) == 0)

    for mod in ob.modifiers:
        if mod.type == 'SUBSURF':
            pass
        elif mod.type == 'MULTIRES':
            pass
        elif mod.type == 'MIRROR':
            if mod.mirror_object == None:
                pass
            else:
                return True
        elif mod.type == 'BEVEL' and no_anim_data:
            pass
        elif mod.type == 'EDGE_SPLIT' and no_anim_data:
            pass
        elif mod.type == 'SOLIDIFY' and no_anim_data:
            pass
        elif mod.type == 'MASK' and no_anim_data:
            pass
        elif mod.type == 'REMESH' and no_anim_data:
            pass
        elif mod.type == 'TRIANGULATE' and no_anim_data:
            pass
        elif mod.type == 'WIREFRAME' and no_anim_data:
            pass
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