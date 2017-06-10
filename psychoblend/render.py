import bpy
import time
import os
import subprocess
import base64
import struct
from . import psy_export

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

    def _start_psychopath(self, scene, psy_filepath, use_stdin, crop):
        psy_binary = PsychopathRender._locate_binary()
        if not psy_binary:
            print("Psychopath: could not execute psychopath, possibly Psychopath isn't installed")
            return False

        # Figure out command line options
        args = []
        if crop != None:
            args += ["--crop", str(crop[0]), str(self.size_y - crop[3]), str(crop[2] - 1), str(self.size_y - crop[1] - 1)]
        if use_stdin:
            args += ["--spb", str(scene.psychopath.max_samples_per_bucket), "--blender_output", "--use_stdin"]
        else:
            args += ["--spb", str(scene.psychopath.max_samples_per_bucket), "--blender_output", "-i", psy_filepath]

        # Start Rendering!
        try:
            self._process = subprocess.Popen([psy_binary] + args, bufsize=1, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        except OSError:
            # TODO, report api
            print("Psychopath: could not execute '%s'" % psy_binary)
            import traceback
            traceback.print_exc()
            print ("***-DONE-***")
            return False

        return True

    def _draw_bucket(self, crop, bucket_info, pixels_encoded):
        if crop != None:
            x = bucket_info[0] - crop[0]
            y = self.size_y - bucket_info[3] - crop[1]
        else:
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
        self._process = None
        try:
            self._render(scene)
        except:
            if self._process != None:
                self._process.terminate()
            raise

    def _render(self, scene):
        # has to be called to update the frame on exporting animations
        scene.frame_set(scene.frame_current)

        export_path = scene.psychopath.export_path.strip()
        use_stdin = False

        r = scene.render
        # compute resolution
        self.size_x = int(r.resolution_x * r.resolution_percentage / 100)
        self.size_y = int(r.resolution_y * r.resolution_percentage / 100)

        # Calculate border cropping, if any.
        if scene.render.use_border == True:
            minx = r.resolution_x * scene.render.border_min_x * (r.resolution_percentage / 100)
            miny = r.resolution_y * scene.render.border_min_y * (r.resolution_percentage / 100)
            maxx = r.resolution_x * scene.render.border_max_x * (r.resolution_percentage / 100)
            maxy = r.resolution_y * scene.render.border_max_y * (r.resolution_percentage / 100)
            crop = (int(minx), int(miny), int(maxx), int(maxy))
        else:
            crop = None

        # Are we using an output file or standard in/out?
        if export_path != "":
            export_path += "_%d.psy" % scene.frame_current
        else:
            # We'll write directly to Psychopath's stdin
            use_stdin = True

        if use_stdin:
            # Start rendering
            if not self._start_psychopath(scene, export_path, use_stdin, crop):
                self.update_stats("", "Psychopath: Not found")
                return

            self.update_stats("", "Psychopath: Collecting...")
            # Export to Psychopath's stdin
            if not psy_export.PsychoExporter(self._process.stdin, self, scene).export_psy():
                # Render cancelled in the middle of exporting,
                # so just return.
                self._process.terminate()
                return
            self._process.stdin.write(bytes("__PSY_EOF__", "utf-8"))
            self._process.stdin.flush()

            self.update_stats("", "Psychopath: Building")
        else:
            # Export to file
            self.update_stats("", "Psychopath: Exporting data from Blender")
            with open(export_path, 'w+b') as f:
                if not psy_export.PsychoExporter(f, self, scene).export_psy():
                    # Render cancelled in the middle of exporting,
                    # so just return.
                    return

            # Start rendering
            self.update_stats("", "Psychopath: Rendering from %s" % export_path)
            if not self._start_psychopath(scene, export_path, use_stdin, crop):
                self.update_stats("", "Psychopath: Not found")
                return

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

            self.update_stats("", "Psychopath: Rendering")

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
                    self._draw_bucket(crop, bucket_info, pixels)

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
