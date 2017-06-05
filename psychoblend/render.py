import bpy
import time
import os
import subprocess
import tempfile
import base64
import struct
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

    def _export(self, scene, export_path):
        exporter = psy_export.PsychoExporter(self, scene)
        return exporter.export_psy(export_path)

    def _render(self, scene, psy_filepath):
        psy_binary = PsychopathRender._locate_binary()
        if not psy_binary:
            print("Psychopath: could not execute psychopath, possibly Psychopath isn't installed")
            return False

        # TODO: figure out command line options
        args = ["--spb", str(scene.psychopath.max_samples_per_bucket), "--blender_output", "-i", psy_filepath]

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

    def _draw_bucket(self, bucket_info, pixels_encoded):
        x = bucket_info[0]
        y = self.size_y - bucket_info[3]
        width = bucket_info[2] - bucket_info[0]
        height = bucket_info[3] - bucket_info[1]

        # Decode pixel data
        pixels = [p for p in struct.iter_unpack("ffff", base64.b64decode(pixels_encoded))]
        pixels_flipped = []
        for i in range(height):
            n = height - i - 1
            pixels_flipped += pixels[n*width:(n+1)*width]

        # Write pixel data to render image
        result = self.begin_result(x, y, width, height)
        lay = result.layers[0].passes["Combined"]
        lay.rect = pixels_flipped
        self.end_result(result)

    def render(self, scene):
        # has to be called to update the frame on exporting animations
        scene.frame_set(scene.frame_current)

        export_path = scene.psychopath.export_path
        if export_path != "":
            export_path += "_%d.psy" % scene.frame_current
        else:
            # Create a temporary file for exporting
            export_path = get_temp_filename('.psy')

        # start export
        self.update_stats("", "Psychopath: Exporting data from Blender")
        if not self._export(scene, export_path):
            # Render cancelled in the middle of exporting,
            # so just return.
            return

        # Start rendering
        self.update_stats("", "Psychopath: Rendering from exported file")
        if not self._render(scene, export_path):
            self.update_stats("", "Psychopath: Not found")
            return

        r = scene.render
        # compute resolution
        self.size_x = int(r.resolution_x * r.resolution_percentage / 100)
        self.size_y = int(r.resolution_y * r.resolution_percentage / 100)

        # If we can, make the render process's stdout non-blocking.  The
        # benefit of this is that canceling the render won't block waiting
        # for the next piece of input.
        try:
            import fcntl
            fd = self._process.stdout.fileno()
            fl = fcntl.fcntl(fd, fcntl.F_GETFL)
            fcntl.fcntl(fd, fcntl.F_SETFL, fl | os.O_NONBLOCK)
        except:
            print("NOTE: Can't make Psychopath's stdout non-blocking, so canceling renders may take a moment to respond.")

        # Process output from rendering process
        reached_first_bucket = False
        output = b""
        render_process_finished = False
        all_output_consumed = False
        while not (render_process_finished and all_output_consumed):
            if self._process.poll() != None:
                render_process_finished = True

            # Check for render cancel
            if self.test_break():
                self._process.terminate()
                break

            # Get render output from stdin
            tmp = self._process.stdout.read1(2**16)
            if len(tmp) == 0:
                time.sleep(0.0001) # Don't spin on the CPU
                if render_process_finished:
                    all_output_consumed = True
                continue
            output += tmp
            outputs = output.split(b'DIV\n')

            # Skip render process output until we hit the first bucket.
            # (The stuff before it is just informational printouts.)
            if not reached_first_bucket:
                if len(outputs) > 1:
                    reached_first_bucket = True
                    outputs = outputs[1:]
                else:
                    continue

            # Clear output buffer, since it's all in 'outputs' now.
            output = b""

            # Process buckets
            for bucket in outputs:
                if len(bucket) == 0:
                    continue

                if bucket[-11:] == b'BUCKET_END\n':
                    # Parse bucket text
                    contents = bucket.split(b'\n')
                    percentage = contents[0]
                    bucket_info = [int(i) for i in contents[1].split(b' ')]
                    pixels = contents[2]

                    # Draw the bucket
                    self._draw_bucket(bucket_info, pixels)

                    # Update render progress bar
                    try:
                        progress = float(percentage[:-1])
                    except ValueError:
                        pass
                    finally:
                        self.update_progress(progress/100)
                else:
                    output += bucket

def register():
    bpy.utils.register_class(PsychopathRender)

def unregister():
    bpy.utils.unregister_class(PsychopathRender)
