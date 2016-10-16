import bpy
import time
import os
import subprocess
import tempfile
from . import psy_export

def get_temp_filename(suffix=""):
    tmpf = tempfile.mkstemp(suffix=suffix, prefix='tmp')
    os.close(tmpf[0])
    return(tmpf[1])

class PsychopathRender(bpy.types.RenderEngine):
    bl_idname = 'PSYCHOPATH_RENDER'
    bl_label = "Psychopath"
    DELAY = 1.0

    @staticmethod
    def _locate_binary():
        addon_prefs = bpy.context.user_preferences.addons[__package__].preferences

        # Use the system preference if its set.
        psy_binary = addon_prefs.filepath_psychopath
        if psy_binary:
            if os.path.exists(psy_binary):
                return psy_binary
            else:
                print("User Preference to psychopath %r NOT FOUND, checking $PATH" % psy_binary)

        # search the path all os's
        psy_binary_default = "psychopath"

        os_path_ls = os.getenv("PATH").split(':') + [""]

        for dir_name in os_path_ls:
            psy_binary = os.path.join(dir_name, psy_binary_default)
            if os.path.exists(psy_binary):
                return psy_binary
        return ""

    def _export(self, scene, export_path, render_image_path):
        exporter = psy_export.PsychoExporter(scene)
        exporter.export_psy(export_path, render_image_path)

    def _render(self, scene, psy_filepath):
        psy_binary = PsychopathRender._locate_binary()
        if not psy_binary:
            print("Psychopath: could not execute psychopath, possibly Psychopath isn't installed")
            return False

        # TODO: figure out command line options
        args = ["--spb", str(scene.psychopath.max_samples_per_bucket), "-i", psy_filepath]

        # Start Rendering!
        try:
            self._process = subprocess.Popen([psy_binary] + args, bufsize=1,
                                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        except OSError:
            # TODO, report api
            print("Psychopath: could not execute '%s'" % psy_binary)
            import traceback
            traceback.print_exc()
            print ("***-DONE-***")
            return False

        return True


    def _cleanup(self):
        # for f in (self._temp_file_in, self._temp_file_ini, self._temp_file_out):
        #     for i in range(5):
        #         try:
        #             os.unlink(f)
        #             break
        #         except OSError:
        #             # Wait a bit before retrying file might be still in use by Blender,
        #             # and Windows does not know how to delete a file in use!
        #             time.sleep(self.DELAY)
        # for i in unpacked_images:
        #     for c in range(5):
        #         try:
        #             os.unlink(i)
        #             break
        #         except OSError:
        #             # Wait a bit before retrying file might be still in use by Blender,
        #             # and Windows does not know how to delete a file in use!
        #             time.sleep(self.DELAY)
        pass

    def render(self, scene):
        # has to be called to update the frame on exporting animations
        scene.frame_set(scene.frame_current)

        export_path = scene.psychopath.export_path
        if export_path != "":
            export_path += "_%d.psy" % scene.frame_current
        else:
            # Create a temporary file for exporting
            export_path = get_temp_filename('.psy')

        # Create a temporary file to render into
        render_image_path = get_temp_filename('.exr')

        # start export
        self.update_stats("", "Psychopath: Exporting data from Blender")
        self._export(scene, export_path, render_image_path)

        # Start rendering
        self.update_stats("", "Psychopath: Rendering from exported file")
        if not self._render(scene, export_path):
            self.update_stats("", "Psychopath: Not found")
            return

        r = scene.render
        # compute resolution
        x = int(r.resolution_x * r.resolution_percentage)
        y = int(r.resolution_y * r.resolution_percentage)

        result = self.begin_result(0, 0, x, y)
        lay = result.layers[0]

        # TODO: Update viewport with render result while rendering
        output = b""
        while self._process.poll() == None:
            # Wait for self.DELAY seconds, but check for render cancels
            # and progress updates while waiting.
            t = 0.0
            while t < self.DELAY:
                # Check for render cancel
                if self.test_break():
                    self._process.terminate()
                    break

                # Update render progress bar
                output += self._process.stdout.read1(2**16)
                outputs = output.rsplit(b'\r')
                progress = 0.0
                if len(outputs) > 0 and outputs[-1][-1] == b"%"[0]:
                    try:
                        progress = float(outputs[-1][:-1])
                    except ValueError:
                        pass
                    finally:
                        self.update_progress(progress/100)

                time.sleep(0.05)
                t += 0.05

            # # Update viewport image with latest render output
            # if os.path.exists(render_image_path):
            #     # This assumes the file has been fully written We wait a bit, just in case!
            #     try:
            #         lay.load_from_file(render_image_path)
            #         self.update_result(result)
            #     except RuntimeError:
            #         pass

        # Load final image
        lay.load_from_file(render_image_path)
        self.end_result(result)

        # Delete temporary image file
        os.remove(render_image_path)

def register():
    bpy.utils.register_class(PsychopathRender)

def unregister():
    bpy.utils.unregister_class(PsychopathRender)
