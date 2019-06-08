#!/usr/bin/env python3

# This file is originally from the supplemental material of the paper
# "Physically Meaningful Rendering using Tristimulus Colours" by Meng et al.
# It has been adapted by Nathan Vegdahl to generate Rust instead of C.
# Only the data tables are generated, and should be put in spectra_tables.rs
# The executable code lives in lib.rs.

import numpy as np
import scipy
import math
import time
import os
import sys

try:
    import colour.plotting as clr
    import colour.recovery as rec
    import colour
    have_colour_package = True
except:
    print("Install colour-science using 'sudo pip install colour-science' to get xy grid plots.")
    print("See http://www.colour-science.org for more information.")
    have_colour_package = False

# Looking at the code, it looks like "Path" is used unconditionally, so
# matplotlib is actually just required.  Import unconditionally.
# --Nathan V
#try:
#print("Install matplotlib to get plots.")
import matplotlib.pyplot as plt
from matplotlib.path import Path
have_matplotlib = True
#except:
#    have_matplotlib = False

# ------------------------------------------------------------------------------
# Color matching functions.
# Note: The load function assumes a CSV input, where each row
# has wavelength, x, y, z (in that order).
# For our paper, we used the truncated set of CMF from 380nm to 780nm, CIE 1931
# standard colorimetric observer, as recommended in CIE Technical Report
# Colorimetry, 2004 (ISBN 3901906339). The CMF values can be found n Table T.4. 
# of the technical report.
#
# The same values can be obtained by downloading 
# the CIE 1931 2-deg. XYZ CMFS here: http://www.cvrl.org/cmfs.htm.
# In the table, use values for wavelengths in [380, 780] and round to 6 
# decimal places.
# ------------------------------------------------------------------------------
class Cmf:
    cmf = []

    @classmethod
    def load(cls, filename):
        cls.cmf = np.loadtxt(filename, delimiter=',')
        assert(cls.cmf.shape[1] == 4)

    @classmethod
    def num_bins(cls):
        return cls.cmf.shape[0]

    @classmethod
    def bin_size(cls):
        return cls.cmf[1,0]-cls.cmf[0,0]

    @classmethod
    def wavelength(cls):
        return cls.cmf[:,0]

    @classmethod
    def x_bar(cls):
        return cls.cmf[:,1]

    @classmethod
    def y_bar(cls):
        return cls.cmf[:,2]

    @classmethod
    def z_bar(cls):
        return cls.cmf[:,3]

    @classmethod
    def xyz_from_spectrum(cls, spectrum):
        '''As CIE instructs, we integrate using simple summation.'''
        assert(cls.cmf.shape[0] == len(spectrum))
        d_lambda = cls.wavelength()[1]-cls.wavelength()[0]

        xyz = [0, 0, 0]
        for x_bar, y_bar, z_bar, s in zip(cls.x_bar(), cls.y_bar(), cls.z_bar(), spectrum):
            xyz[0] += x_bar * s
            xyz[1] += y_bar * s
            xyz[2] += z_bar * s

        return [v * d_lambda for v in xyz]

    @classmethod
    def xyz_ee_white(cls):
        ee_white = [1] * cls.cmf.shape[0]
        return cls.xyz_from_spectrum(ee_white)

# ------------------------------------------------------------------------------
# Transform between color spaces.
# ------------------------------------------------------------------------------
class Transform:
    # --------------------------------------------------------------------------
    # Homogenize/dehomogenize vectors.
    # --------------------------------------------------------------------------
    @staticmethod
    def hom(v2):
        assert(len(v2) >= 2)
        return np.matrix([[v2[0]], [v2[1]], [1]])

    @staticmethod
    def dehom(v3):
        assert((v3.shape[0] == 3 and v3.shape[1] == 1)
            or (v3.shape[0] == 1 and v3.shape[1] == 3))
        v = v3.flatten().tolist()[0]
        return [v[0]/v[2], v[1]/v[2]]
    

    # ------------------------------------------------------------------------------
    # Convert from xyy to xyz and back.
    # ------------------------------------------------------------------------------
    @staticmethod
    def xyz_from_xyy(xyy):
        return (xyy[0] * xyy[2]/xyy[1], 
                xyy[2], 
                (1-xyy[0]-xyy[1]) * xyy[2]/xyy[1])


    @staticmethod
    def xyy_from_xyz(xyz):
        s = sum(xyz)
        return (xyz[0] / s, xyz[1] / s, xyz[1])

    # ------------------------------------------------------------------------------
    # Convert from srgb to xyz and back.
    # ------------------------------------------------------------------------------
    def xyz_from_srgb(srgb):
        # This matrix is computed by transforming the sRGB primaries into xyz.
        # Primaries are 
        # red:   xy = 0.64, Y = 0.2126
        # green: xy = 0.30, Y = 0.7152
        # blue:  xy = 0.15, Y = 0.0722,
        # where the luminance values are taken from HDTV Recommendation BT.709
        # http://www.itu.int/rec/R-REC-BT.709/en
        M = np.matrix([[ 0.41231515,  0.3576,      0.1805    ]
                       [ 0.2126    ,  0.7152,      0.0722    ]
                       [ 0.01932727,  0.1192,      0.95063333]])
        return np.dot(M, srgb)

    def srgb_from_xyz(xyz):
        # This is the inverse of the above matrix.
        M = np.matrix([[ 3.24156456, -1.53766524, -0.49870224],
                       [-0.96920119,  1.87588535,  0.04155324],
                       [ 0.05562416, -0.20395525,  1.05685902]])
        return np.dot(M, xyz)

    # EE-white adapted sRGB (Smits uses this).
    def xyz_from_ergb(ergb):
        M = np.matrix([
            [0.496859,  0.339094,  0.164047],
            [0.256193,  0.678188,  0.065619],
            [0.023290,  0.113031,  0.863978]
        ])
        return np.dot(M, xyz)

    # ------------------------------------------------------------------------------
    # Convert from xy to xy* and back.
    # ------------------------------------------------------------------------------
    mat_xystar_to_xy = None
    mat_xy_to_xystar = None

    @classmethod
    def init_xystar(cls):
        '''xy* is a color space where the line between blue and red is horizontal.
           Also, equal-energy white is the origin.
           xy* depends only on the color matching functions used.'''
        num_bins = len(Cmf.wavelength())

        # Pure blue.
        s = [0] * num_bins
        s[0] = 1
        xy0 = cls.xyy_from_xyz(Cmf.xyz_from_spectrum(s))

        # Pure red.
        s = [0] * num_bins
        s[-1] = 1
        xy1 = cls.xyy_from_xyz(Cmf.xyz_from_spectrum(s))

        d = np.array(xy1[:2])-np.array(xy0[:2])
        d /= math.sqrt(np.vdot(d, d))

        # Translation to make ee-white (in xy) the origin.
        T = np.matrix([[ 1, 0, -1/3],
                       [ 0, 1, -1/3],
                       [ 0, 0,    1]])
        # Rotation to make purple line horizontal.
        R = np.matrix([[ d[0], d[1], 0],
                       [-d[1], d[0], 0],
                       [    0,    0, 1]]) 

        cls.mat_xy_to_xystar = np.dot(R, T)
        cls.mat_xystar_to_xy = cls.mat_xy_to_xystar.getI()

    @classmethod
    def xystar_from_xy(cls, xy):
        return cls.dehom(np.dot(cls.mat_xy_to_xystar, cls.hom(xy)))

    @classmethod
    def xy_from_xystar(cls, xystar):
        return cls.dehom(np.dot(cls.mat_xystar_to_xy, cls.hom(xystar)))

    # ------------------------------------------------------------------------------
    # Convert from xy to uv and back.
    # ------------------------------------------------------------------------------
    mat_uv_to_xystar = None
    mat_xystar_to_uv = None
    mat_uv_to_xy     = None
    mat_xy_to_uv     = None

    @classmethod
    def init_uv(cls, xystar_bbox, grid_res):
        '''uv is derived from xy* by transforming grid points to integer coordinates.
           uv depends on xy* and the grid used.'''

        # Translate xystar bounding box min to origin.
        T = np.matrix([[1, 0, -xystar_bbox[0][0]],
                       [0, 1, -xystar_bbox[0][1]],
                       [0, 0,                1]])

        # Scale so that one grid cell has unit size.
        w = xystar_bbox[1][0]-xystar_bbox[0][0]
        h = xystar_bbox[1][1]-xystar_bbox[0][1]
        S = np.matrix([[grid_res[0] / w, 0, 0],
                       [0, grid_res[1] / h, 0],
                       [0, 0, 1]])

        cls.mat_xystar_to_uv = np.dot(S, T)
        cls.mat_uv_to_xystar = cls.mat_xystar_to_uv.getI()
        cls.mat_xy_to_uv     = np.dot(cls.mat_xystar_to_uv, cls.mat_xy_to_xystar)
        cls.mat_uv_to_xy     = cls.mat_xy_to_uv.getI()

    @classmethod
    def uv_from_xy(cls, xy):
        return cls.dehom(np.dot(cls.mat_xy_to_uv, cls.hom(xy)))

    @classmethod
    def xy_from_uv(cls, uv):
        return cls.dehom(np.dot(cls.mat_uv_to_xy, cls.hom(uv)))

    @classmethod
    def uv_from_xystar(cls, xystar):
        return cls.dehom(np.dot(cls.mat_xystar_to_uv, cls.hom(xystar)))

    @classmethod
    def xystar_from_uv(cls, uv):
        return cls.dehom(np.dot(cls.mat_uv_to_xystar, cls.hom(uv)))

# ------------------------------------------------------------------------------
# Compute functor for all elements of data using a process pool, and call 
# finished with (i, result) afterwards.
# ------------------------------------------------------------------------------
def multiprocess_progress(data, functor, finished, data_size, early_clip=None):
    from multiprocessing import Process, current_process, Queue

    num_procs = os.cpu_count()-1

    def worker(wnum, input_queue, output_queue):
        os.sched_setaffinity(0, [wnum])
        while True:
            try:
                idx, value = input_queue.get(block=False)
                if value == 'STOP':
                    break
                output_queue.put((idx, functor(value)))
            except:
                pass
            os.sched_yield()

    task_queue = Queue(2*num_procs)
    done_queue = Queue(2*num_procs)

    # Launch workers.
    print('Running {} workers ...'.format(num_procs))
    processes = []
    for i in range(num_procs):
        processes.append(Process(target = worker,
            args = (i, task_queue, done_queue),
            name = 'worker {}'.format(i),
            daemon = True))
        processes[-1].start()

    # Push input data, and check for output data.
    num_sent = 0
    num_done = 0
    num_clipped = 0
    iterator = iter(data)
    perc = 0

    def print_progress(msg=None):
        msg_str = ''
        if msg is not None:
            msg_str = '['+msg+']'
        print('\033[2K\r{} sent, {} done, {} clipped, {} total ({} %) {}'.format(num_sent, 
            num_done, num_clipped, data_size, perc, msg_str), end='')

    while num_done < data_size:
        print_progress('sending work')

        while num_sent < data_size and not task_queue.full():
            nextval = next(iterator)
            clipped = False
            if early_clip is not None:
                clipped, clip_result = early_clip(num_sent, nextval)
                if clipped:
                    finished(num_sent, clip_result)
                    num_clipped += 1
                    num_done += 1

            if not clipped:
                task_queue.put((num_sent, nextval))

            num_sent += 1
            os.sched_yield()

        while True:
            try:
                i, result = done_queue.get(block=False)
                finished(i, result)
                num_done += 1
                perc = int(num_done / data_size * 100)
                print_progress('collecting results')
            except:
                break;
            time.sleep(0)

        print_progress()
        time.sleep(0)

    # Terminate workers.
    for i in range(num_procs):
        task_queue.put((-1, 'STOP'))

    for p in processes:
        p.join()

    print('\n ... done')

# ------------------------------------------------------------------------------
# Given a color in XYZ, determine a smooth spectrum that corresponds to that 
# color.
# ------------------------------------------------------------------------------
def find_spectrum(xyz):
    from scipy.optimize import minimize
    
    # As an objective, we use a similar roughness term as Smits did.
    def objective(S):
        roughness = 0
        for i in range(len(S)-1):
            roughness += (S[i]-S[i+1])**2
        # Note: We found much better convergence with the square term!
        # roughness = math.sqrt(roughness)
        return roughness

    num_bins = Cmf.num_bins()
    x0       = [1] * num_bins
    
    # Constraint: Match XYZ values.
    cnstr = { 
        'type': 'eq', 
        'fun': lambda s: (np.array(Cmf.xyz_from_spectrum(s))-xyz)
    }

    # We want positive spectra.
    bnds = [(0, 1000)] * num_bins
    
    res = minimize(objective, x0, method='SLSQP', constraints=cnstr, 
                   bounds=bnds, options={"maxiter": 2000, "ftol": 1e-10})
    if not res.success:
        err_message = 'Error for xyz={} after {} iterations: {}'.format(xyz, res.nit, res.message)
        return ([0] * num_bins, True, err_message)
    else:
        # The result may contain some very tiny negative values due 
        # to numerical issues. Clamp those to 0.
        return ([max(x, 0) for x in res.x], False, "")


# ------------------------------------------------------------------------------
# Get the boundary of the horseshoe as a path in xy*.
# ------------------------------------------------------------------------------
def horseshoe_path():
    verts = []
    codes = []

    d_lambda = Cmf.wavelength()[1]-Cmf.wavelength()[0]
    for x, y, z in zip(Cmf.x_bar(), Cmf.y_bar(), Cmf.z_bar()):
        xyz    = [x*d_lambda, y*d_lambda, z*d_lambda]
        xyY    = Transform.xyy_from_xyz(xyz)
        xystar = Transform.xystar_from_xy(xyY[:2])
        verts.append(xystar)
        codes.append(Path.LINETO)

    codes[0] = Path.MOVETO
    codes.append(Path.CLOSEPOLY)

    vx = [x for (x, y) in verts]
    vy = [y for (x, y) in verts]
    bbox = [ (min(vx), min(vy)), (max(vx), max(vy)) ]

    verts.append((0,0))
    return (Path(verts, codes), bbox)

# ------------------------------------------------------------------------------
# Grid data structures.
# ------------------------------------------------------------------------------

class DataPoint:
    def __init__(self):
        self.xystar             = (0, 0)
        self.uv                 = (0, 0)
        self.Y                  = 0
        self.spectrum           = [0]
        self.M                  = 0
        self.inside             = False
        self.equal_energy_white = False
        self.broken             = False

    def update_uv(self):
        self.uv = Transform.uv_from_xystar(self.xystar)

class GridCell:
    def __init__(self):
        self.indices   = []
        self.triangles = []
        self.inside    = True

# binary search to find intersection
def find_intersection(p0, p1, i0, i1, clip_path):
    delta = p1-p0
    if np.linalg.norm(delta) < 0.0001:
        # Points are very close, terminate.
        # Move new intersection slightly into the gamut.
        delta *= 0.998
        if i0:
            return p1 - delta
        else:
            return p0 + delta

    p01 = 0.5 * (p0 + p1)
    i01 = clip_path.contains_point(p01)
    if i0 != i01:
        return find_intersection(p0, p01, i0, i01, clip_path)
    elif i1 != i01:
        return find_intersection(p01, p1, i01, i1, clip_path)
    else:
        print ("something wrong here")
        return p01

def clip_edge(d0, d1, clip_path):
    from operator import xor
    if not xor(d0.inside, d1.inside):
        return (False, None)

    p0 = np.array(d0.xystar)
    p1 = np.array(d1.xystar)
    p  = find_intersection(p0, p1, d0.inside, d1.inside, clip_path)
    
    data_point        = DataPoint()
    data_point.xystar = p
    data_point.inside = True

    return (True, data_point)

def generate_xystar_grid(scale):
    print("Generating clip path ...")
    clip_path, bbox = horseshoe_path()

    # We know that xy(1/3, 1/3) = xystar(0, 0) must be a grid point.
    # subdivide the rectangle between that and the purest red regularly with res.
    # Note: This can be freely chosen, but we found 6,4 to be a reasonable value.
    res          = (6, 4)
    white_xystar = [0, 0]
    step_x       = abs(white_xystar[0]-bbox[1][0]) / res[0]
    step_y       = abs(white_xystar[1]-bbox[0][1]) / res[1]

    # Find bbox top left corner so that the whole diagram is contained.
    add_x = int(math.ceil(abs(white_xystar[0]-bbox[0][0]) / step_x))
    add_y = int(math.ceil(abs(bbox[1][1]-white_xystar[1]) / step_y))

    # The index of white - we will set this spectrum to equal energy white.
    white_idx = (add_x, res[1])

    grid_res = (res[0] + add_x, res[1] + add_y)
    bbox = [
        # min
        (white_xystar[0]- step_x * add_x, bbox[0][1]),
        # max
        (bbox[1][0], white_xystar[1] + step_y * add_y)
    ]

    grid        = [GridCell() for i in range(grid_res[0] * grid_res[1])]
    data_points = []

    # Generate grid points.
    print(" Generating grid points in xy* ...")
    for (x,y) in [(x,y) for y in range(grid_res[1]+1) for x in range(grid_res[0]+1)]:
        data_point        = DataPoint()
        data_point.xystar = (bbox[0][0] + step_x * x, bbox[0][1] + step_y * y)

        if (x, y) == white_idx:
            # Numerically, we want the white point to be at xy = (1/3, 1/3).
            delta = np.array(data_point.xystar) - np.array(white_xystar)
            assert(np.dot(delta, delta) < 1e-7)
            data_point.equal_energy_white = True

        # Clip on horseshoe.
        if clip_path.contains_point(data_point.xystar) \
            or (x > 0 and y == 0): # Special case for purple line.
            data_point.inside = True

        new_idx = len(data_points)
        data_points.append(data_point)
    
        # Add new index to this all four adjacent cells.
        for (cx, cy) in [(x-dx, y-dy) for dy in range(2) for dx in range(2)]:
            if cx >= 0 and cx < grid_res[0] and cy >= 0 and cy < grid_res[1]:
                cell = grid[cy * grid_res[0] + cx]
                cell.indices.append(new_idx)
                cell.inside = cell.inside and data_point.inside

    # Clip grid cells against horseshoe.
    print(" Clipping cells to xy gamut ...")
    for (x, y) in [(x, y) for x in range(grid_res[0]) for y in range(grid_res[1])]:
        cell = grid[y * grid_res[0] + x]

        # No need to clip cells that are completely inside.
        if cell.inside:
            continue

        # We clip the two outgoing edges of each point:
        #
        # d2
        #  .
        # d0 . d1
        # Note: We assume here that data_points was generated as a regular
        #       grid in row major order.
        d0 = data_points[(y+0)*(grid_res[0]+1)+(x+0)]
        d1 = data_points[(y+0)*(grid_res[0]+1)+(x+1)]
        d2 = data_points[(y+1)*(grid_res[0]+1)+(x+0)]

        (clipped_h, p_h) = clip_edge(d0, d1, clip_path)
        if clipped_h:
            new_idx = len(data_points)
            data_points.append(p_h)
            cell.indices.append(new_idx)
            if y > 0:
                grid[(y-1) * grid_res[0] + x].indices.append(new_idx)

        (clipped_v, p_v) = clip_edge(d0, d2, clip_path)
        if clipped_v:
            new_idx = len(data_points)
            data_points.append(p_v)
            cell.indices.append(new_idx)
            if x > 0:
                grid[y * grid_res[0] + x - 1].indices.append(new_idx)

    # Compact grid points (throw away points that are not inside).
    print(" Compacting grid ...")
    new_data_points = []
    new_indices = []
    prefix = 0
    for data_point in data_points:
        if data_point.inside:
            new_indices.append(prefix)
            new_data_points.append(data_point)
            prefix += 1
        else:
            new_indices.append(-1)
    data_points = new_data_points

    for gridcell in grid:
        new_cell_indices = []
        for index in range(len(gridcell.indices)):
            old_index = gridcell.indices[index]
            if new_indices[old_index] >= 0:
                new_cell_indices.append(new_indices[old_index])
        gridcell.indices = new_cell_indices[:]

    # Scale points down towards white point to avoid singular spectra.
    for data_point in data_points:
        data_point.xystar = [v * scale for v in data_point.xystar]

    bbox[0] = [v * scale for v in bbox[0]]
    bbox[1] = [v * scale for v in bbox[1]]

    return data_points, grid, grid_res, bbox

# Plot the grid.
def plot_grid(filename, data_points, grid, bbox_xystar, xystar=True):
    if not have_matplotlib or not have_colour_package:
        return

    print("Plotting the grid ...")

    plt.figure()
    # Draw a nice chromaticity diagram.
    clr.CIE_1931_chromaticity_diagram_plot(standalone=False)
    clr.canvas(figure_size=(7,7))

    # Show the sRGB gamut.
    color_space = clr.get_RGB_colourspace('sRGB')
    x = color_space.primaries[:,0].tolist()
    y = color_space.primaries[:,1].tolist()
    plt.fill(x, y, color='black', label='sRGB', fill=False)

    # Draw crosses into all internal grid cells.
    # for gridcell in grid:
    #     if len(gridcell.indices) > 0 and gridcell.inside:
    #         if xystar:
    #             pointx = sum([data_points[i].xystar[0] for i in gridcell.indices])
    #             pointy = sum([data_points[i].xystar[1] for i in gridcell.indices])
    #             pointx /= len(gridcell.indices)
    #             pointy /= len(gridcell.indices)
    #             (pointx, pointy) = Transform.xy_from_xystar((pointx, pointy))
    #             plt.plot(pointx, pointy, "x", color="black")
    #         else:
    #             pointx = sum([data_points[i].uv[0] for i in gridcell.indices])
    #             pointy = sum([data_points[i].uv[1] for i in gridcell.indices])
    #             pointx /= len(gridcell.indices)
    #             pointy /= len(gridcell.indices)
    #             (pointx, pointy) = Transform.xy_from_uv((pointx, pointy))
    #             plt.plot(pointx, pointy, "x", color="black")
 
    # Draw data points.
    for i, data_point in enumerate(data_points):
        if xystar:
            p = Transform.xy_from_xystar(data_point.xystar)
        else:
            p = Transform.xy_from_uv(data_point.uv)

        if data_point.equal_energy_white:
            plt.plot(p[0], p[1], "o", color="white", ms=4)
        elif data_point.broken:
            plt.plot(p[0], p[1], "o", color="red", ms=4)
        else:
            plt.plot(p[0], p[1], "o", color="green", ms=4)

        # Show grid point indices, for debugging.
        # plt.text(p[0]+0.01, p[1]-0.01, '{}'.format(i))

    bp0 = Transform.xy_from_xystar([bbox_xystar[0][0], bbox_xystar[0][1]])
    bp1 = Transform.xy_from_xystar([bbox_xystar[0][0], bbox_xystar[1][1]])
    bp2 = Transform.xy_from_xystar([bbox_xystar[1][0], bbox_xystar[1][1]])
    bp3 = Transform.xy_from_xystar([bbox_xystar[1][0], bbox_xystar[0][1]])
    plt.plot([bp0[0], bp1[0], bp2[0], bp3[0], bp0[0]],
             [bp0[1], bp1[1], bp2[1], bp3[1], bp0[1]],
             label="Grid Bounding Box")

    plt.xlabel('$x$')
    plt.ylabel('$y$')

    plt.legend()
    plt.savefig(filename)

# ------------------------------------------------------------------------------
# Compute spectra for all data points.
# ------------------------------------------------------------------------------
def compute_spectrum(data_point):
    xy = Transform.xy_from_uv(data_point.uv)

    # Set luminance to y. This means that X+Y+Z = 1, 
    # since y = Y / (X+Y+Z) = y / (X+Y+Z).
    xyY = [xy[0], xy[1], xy[1]]
    xyz = Transform.xyz_from_xyy(xyY)

    spectrum = []
    broken   = False

    if data_point.equal_energy_white:
        # Since we want X=Y=Z=1/3 (so that X+Y+Z=1), the equal-energy white 
        # spectrum we want is 1/(3 int(x)) for x color matching function.
        spectrum = [1 / (3 * Cmf.xyz_ee_white()[0])] * Cmf.num_bins()
    else:
        spectrum, broken, message = find_spectrum(xyz)

        if broken:
            print("Couldn't find a spectrum for uv=({uv[0]},{uv[1]})".format(uv=data_point.uv))
            print(message)

    xyz = Cmf.xyz_from_spectrum(spectrum)
    sum = xyz[0] + xyz[1] + xyz[2]
    if sum > 1.01 or sum < 0.99:
        print('Invalid brightness {} for uv=({uv[0]},{uv[1]})'.format(sum, uv=data_point.uv))

    return (spectrum, broken)


# ------------------------------------------------------------------------------

def compute_spectra(data_points):
    print('Computing spectra ...')

    def finished(i, result):
        data_points[i].spectrum = result[0]
        data_points[i].broken   = result[1]

    multiprocess_progress(data_points, compute_spectrum, finished, len(data_points))


# ------------------------------------------------------------------------------
# Plot some of our fitted spectra.
# Plot to multiple output files, since there are so many spectra.
# ------------------------------------------------------------------------------
def plot_spectra(data_points):
    if not have_matplotlib or not have_colour_package:
        return

    print('Plotting spectra ...')
    plots_per_file = 15

    #plt.figure(figsize=(12, 16))

    cur_page = -1
    ax_shape = (17, 4)
    axes = None
    for i, data_point in enumerate(data_points):
        page_size =(ax_shape[0]*ax_shape[1])
        page = i // page_size
        if page > cur_page:
            if cur_page >= 0:
                plt.savefig('spectra_{}.svg'.format(cur_page))
            fig, axes = plt.subplots(ax_shape[0], ax_shape[1], figsize=(14, 18))
            cur_page = page

        j = i % page_size
        row = j % axes.shape[0]
        col = j // axes.shape[0]
        print(row, col)

        if row >= axes.shape[0] or col >= axes.shape[1]:
            print('cannot plot spectrum', i)
            continue

        ax = axes[row,col]

        xy = Transform.xy_from_uv(data_point.uv)
        # take a copy, we're going to normalise it
        s = data_point.spectrum[:]
        max_val = 0
        for j in range(len(s)):
            if s[j] > max_val:
                max_val = s[j];
        if max_val > 0:
            for j in range(len(s)):
                s[j] = s[j]/max_val
        ax.plot(Cmf.wavelength(), s, color='black', lw=2)
        ax.set_ylim(-0.01, 1.1)
        ax.set_yticklabels([])
        ax.set_xticklabels([])

        perc = int((i+1) / len(data_points) * 100)
        print(' {} / {} ({} %)             \r'.format((i+1), len(data_points), perc), end='')
    plt.savefig('spectra_{}.svg'.format(cur_page))

    print('\n... done')

# ------------------------------------------------------------------------------
# Write spectral data
# ------------------------------------------------------------------------------
def write_output(data_points, grid, grid_res, filename):
    print('Write output ...')
    with open(filename, 'w') as f:
        lambda_min       = Cmf.wavelength()[0]
        lambda_max       = Cmf.wavelength()[-1]
        num_spec_samples = Cmf.num_bins()
        spec_bin_size    = Cmf.bin_size()
        
        f.write('// This file is auto-generated by generate_spectra_tables.py\n')
        f.write('#![allow(dead_code)]\n')
        f.write('#![cfg_attr(rustfmt, rustfmt_skip)]\n')
        f.write('#![allow(clippy::unreadable_literal)]\n')
        f.write('#![allow(clippy::excessive_precision)]\n')
        f.write('\n')
        f.write('/// This is 1 over the integral over either CMF.\n')
        f.write('/// Spectra can be mapped so that xyz=(1,1,1) is converted to constant 1 by\n')
        f.write('/// dividing by this value. This is important for valid reflectances.\n')
        f.write('pub const EQUAL_ENERGY_REFLECTANCE: f32 = {};'.format(1/max(Cmf.xyz_ee_white())));
        
        f.write('\n\n')
        f.write('// Basic info on the spectrum grid.\n')
        f.write('pub(crate) const SPECTRUM_GRID_WIDTH: i32 = {};\n'.format(grid_res[0]))
        f.write('pub(crate) const SPECTRUM_GRID_HEIGHT: i32 = {};\n'.format(grid_res[1]))
        f.write('\n')
        
        f.write('// The spectra here have these properties.\n')
        f.write('pub const SPECTRUM_SAMPLE_MIN: f32 = {};\n'.format(lambda_min))
        f.write('pub const SPECTRUM_SAMPLE_MAX: f32 = {};\n'.format(lambda_max))
        f.write('pub(crate) const SPECTRUM_BIN_SIZE: f32 = {};\n'.format(spec_bin_size))
        f.write('pub(crate) const SPECTRUM_NUM_SAMPLES: i32 = {};\n'.format(num_spec_samples))
        f.write('\n')

        # Conversion routines xy->xystar and xy->uv and back.
        f.write('// xy* color space.\n')
        f.write('pub(crate) const SPECTRUM_MAT_XY_TO_XYSTAR: [f32; 6] = [\n')
        f.write('    {m[0]}, {m[1]}, {m[2]},\n    {m[3]}, {m[4]}, {m[5]}\n'
            .format(m=Transform.mat_xy_to_xystar[:2,:].flatten().tolist()[0]))
        f.write('];\n')
        f.write('pub(crate) const SPECTRUM_MAT_XYSTAR_TO_XY: [f32; 6] = [\n')
        f.write('    {m[0]}, {m[1]}, {m[2]},\n    {m[3]}, {m[4]}, {m[5]}\n'
            .format(m=Transform.mat_xystar_to_xy[:2,:].flatten().tolist()[0]))
        f.write('];\n')
        
        f.write('// uv color space.\n')
        f.write('pub(crate) const SPECTRUM_MAT_XY_TO_UV: [f32; 6] = [\n')
        f.write('    {m[0]}, {m[1]}, {m[2]},\n    {m[3]}, {m[4]}, {m[5]}\n'
            .format(m=Transform.mat_xy_to_uv[:2,:].flatten().tolist()[0]))
        f.write('];\n')
        f.write('pub(crate) const SPECTRUM_MAT_UV_TO_XY: [f32; 6] = [\n')
        f.write('    {m[0]}, {m[1]}, {m[2]},\n    {m[3]}, {m[4]}, {m[5]}\n'
            .format(m=Transform.mat_uv_to_xy[:2,:].flatten().tolist()[0]))
        f.write('];\n')
        
        f.write('// Grid cells. Laid out in row-major format.\n')
        f.write('// num_points = 0 for cells without data points.\n')
        f.write('#[derive(Copy, Clone)]\n')
        f.write('pub(crate) struct SpectrumGridCell {\n')
        f.write('    pub inside: bool,\n')
        f.write('    pub num_points: i32,\n')
        max_num_idx = 0
        for c in grid:
            if len(c.indices) > max_num_idx:
                max_num_idx = len(c.indices)
        f.write('    pub idx: [i32; {}],\n'.format(max_num_idx))
        f.write('}\n\n')
        
        # Count grid cells
        grid_cell_count = 0
        for (x, y) in [(x,y) for y in range(grid_res[1]) for x in range(grid_res[0])]:
            grid_cell_count += 1
        
        # Write grid cells
        f.write('pub(crate) const SPECTRUM_GRID: [SpectrumGridCell; {}] = [\n'.format(grid_cell_count))
        cell_strings = []
        for (x, y) in [(x,y) for y in range(grid_res[1]) for x in range(grid_res[0])]:
            cell = grid[y * grid_res[0] + x]
            # pad cell indices with -1.
            padded_indices = cell.indices[:] + [-1] * (max_num_idx-len(cell.indices))
                    
            num_inside = len(cell.indices)
            if num_inside > 0:
                idx_str = ', '.join(map(str, padded_indices))
                if cell.inside and num_inside == 4:
                    cell_strings.append('    SpectrumGridCell {{ inside: true, num_points: {}, idx: [{}] }}'.format(num_inside, idx_str))
                else:
                    cell_strings.append('    SpectrumGridCell {{ inside: false, num_points: {}, idx: [{}] }}'.format(num_inside, idx_str))
            else:
                cell_strings.append('    SpectrumGridCell {{ inside: false, num_points: 0, idx: [{}] }}'.format(', '.join(['-1'] * max_num_idx)))
        f.write(',\n'.join(cell_strings))
        f.write('\n];\n\n')
        
        f.write('// Grid data points.\n')
        f.write('#[derive(Copy, Clone)]\n')
        f.write('pub(crate) struct SpectrumDataPoint {\n')
        f.write('    pub xystar: (f32, f32),\n')
        f.write('    pub uv: (f32, f32),\n')
        f.write('    pub spectrum: [f32; {}], // X+Y+Z = 1\n'.format(num_spec_samples))
        f.write('}\n\n')
        data_point_strings = []
        data_point_count = 0
        for p in data_points:
            data_point_count += 1
            spec_str = ', '.join(["{:f}".format(v) for v in list(p.spectrum)])
            data_point_strings.append(
                "    SpectrumDataPoint {{\n"
                "        xystar: ({p.xystar[0]}, {p.xystar[1]}),\n"
                "        uv: ({p.uv[0]}, {p.uv[1]}),\n"
                "        spectrum: [{spec}],\n"
                "    }}".format(p=p, spec=spec_str)
            )
        f.write('pub(crate) const SPECTRUM_DATA_POINTS: [SpectrumDataPoint; {}] = [\n'.format(data_point_count))
        f.write(',\n'.join(data_point_strings))
        f.write('\n];\n\n')


        f.write('// Color matching functions.\n')
        f.write('pub(crate) const CMF_WAVELENGTH: [f32; {}] = [\n'.format(len(Cmf.wavelength())))
        f.write('    {}\n'.format(', '.join(str(v) for v in Cmf.wavelength())))
        f.write('];\n')
        f.write('pub(crate) const CMF_X: [f32; {}] = [\n'.format(len(Cmf.x_bar())))
        f.write('    {}\n'.format(', '.join(str(v) for v in Cmf.x_bar())))
        f.write('];\n')
        f.write('pub(crate) const CMF_Y: [f32; {}] = [\n'.format(len(Cmf.y_bar())))
        f.write('    {}\n'.format(', '.join(str(v) for v in Cmf.y_bar())))
        f.write('];\n')
        f.write('pub(crate) const CMF_Z: [f32; {}] = [\n'.format(len(Cmf.z_bar())))
        f.write('    {}\n'.format(', '.join(str(v) for v in Cmf.z_bar())))
        f.write('];\n\n')
    
    print(' ... done')

# ------------------------------------------------------------------------------
# We need to triangulate along the spectral locus, since our regular grid
# cannot properly capture this edge.
# ------------------------------------------------------------------------------
def create_triangle_fans(grid):
    print("generating triangle fans...")
    for cell in grid:
        num_points = len(cell.indices)
        # skip trivial inner cells (full quad interpolation)\n",
        if len(cell.indices) == 4 and cell.inside:
            # these could be sorted here, too. but turns out we always get them in scanline order
            # so we will know exactly how to treat them in the exported c code.
            continue

        # triangulate hard cases (irregular quads + n-gons, 5-gons in practice)
        if num_points > 0:
            # used for delaunay or plotting:\n",
            points = np.array([data_points[cell.indices[i]].xystar for i in range(num_points)])
            centroid = (sum(points[:,0])/num_points, sum(points[:,1])/num_points)
            dp = DataPoint()
            dp.xystar = centroid
            dp.update_uv()
            index = len(data_points)
            data_points.append(dp)

            # create triangle fan:
            pts = [(points[i], i, cell.indices[i], math.atan2((points[i]-centroid)[1], (points[i]-centroid)[0])) for i in range(num_points)]
            pts = sorted(pts, key=lambda pt: pt[3])
            # print('sorted {}'.format([pts[i][2] for i in range(num_points)]))
            cell.indices = [index] + [pts[i][2] for i in range(num_points)]
            # print('indices: {}'.format(cell.indices))
            num_points = num_points + 1;
            # do that again with the new sort order:
            # points = np.array([data_points[cell.indices[i]].xystar for i in range(num_points)])
            # now try triangle fan again with right pivot
            cell.triangles = [[0, i+1, i+2] for i in range(len(cell.indices)-2)]

# ------------------------------------------------------------------------------
# Compute a high-resolution reflectance map. This map contains, for all
# possible values of (xy), the largest value Y for which the corresponding 
# spectrum is still a valid reflectance.
# ------------------------------------------------------------------------------
def compute_max_brightness(point):
    x           = point[0]
    y           = point[1]

    try:
        xyz = Transform.xyz_from_xyy((x, y, y)) # x+y+z = 1
        spec, broken, msg = find_spectrum(xyz)
        if broken: 
            print('{},{}: {}'.format(x, y, msg))
            return -1

        return 1.0/(106.8 * max(spec))
    except:
        print('Exception - this is fatal.')
        raise

def compute_reflectance_map(res):
    width      = res
    height     = res
    num_pixels = width * height
    buffer     = [0, 0, 0.1] * num_pixels

    def store_buffer():
        with open('reflectance_map.pfm', 'wb') as file:
            import struct
            header = 'PF\n{w} {h}\n{le}\n'.format(
                w  = width, 
                h  = height, 
                le = -1 if sys.byteorder == 'little' else 1)

            s = struct.pack('f' * len(buffer), *buffer)
            file.write(bytes(header, encoding='utf-8'))
            file.write(s)
            file.close()

    def coordinates():
        for y in range(height):
            for x in range(width):
                yield (x / width, y / height)

    def store_pixel(i, v):
        global last_time_stored

        if v == 0:
            pass
        elif v < 0:
            buffer[3*i]   = -v
            buffer[3*i+1] = 0
            buffer[3*i+2] = 0
        else:
            buffer[3*i]   = v
            buffer[3*i+1] = v
            buffer[3*i+2] = v

        now = time.time()
        if (now-last_time_stored) > 60:
            store_buffer()
            last_time_stored = time.time()

    def early_clip(idx, v):
        global clip_path
        if clip_path.contains_point(Transform.xystar_from_xy(v)):
            return (False, 0)
        return (True, 0)

    multiprocess_progress(coordinates(),
        compute_max_brightness,
        store_pixel,
        width*height, 
        early_clip)

    store_buffer()

if __name__ == "__main__":
    # Parse command line options.
    import argparse
    parser = argparse.ArgumentParser(description='Generate spectrum_grid.h')
    parser.add_argument('-s', '--scale', metavar='SCALE', type=float, default=0.97,
        dest='scale',
        help='Scale grid points toward the EE white point using this factor. Defaults to 0.99.')

    parser.add_argument('-p', '--plot_spectra', default=False, action='store_const',
        const=True, dest='plot',
        help='Plot all spectra in a set of png files. Instructive, but takes quite a while.')

    parser.add_argument('-r', '--reflectance_map', metavar='RES', type=int, default=0, 
        dest='reflectance_map',
        help='Generate a high-resolution reflectance map instead of the grid header.')

    parser.add_argument('cmf', metavar='CMF', type=str, help='The cmf file to be used.')

    args = parser.parse_args()

    # Init xystar.
    Cmf.load(args.cmf)
    Transform.init_xystar()

    last_time_stored = 0
    clip_path,_ =  horseshoe_path()

    # plot spectral locus
    # for i in range(0,Cmf.num_bins()):
    #    print('{} {} {}'.format(Cmf.wavelength()[i],
    #             Cmf.x_bar()[i]/(Cmf.x_bar()[i]+Cmf.y_bar()[i]+Cmf.z_bar()[i]),
    #             Cmf.y_bar()[i]/(Cmf.x_bar()[i]+Cmf.y_bar()[i]+Cmf.z_bar()[i])))

    if args.reflectance_map > 0:
        compute_reflectance_map(args.reflectance_map)

    else:
        # Generate the grid.
        data_points, grid, grid_res, xystar_bbox = generate_xystar_grid(args.scale)

        # Init uv.
        Transform.init_uv(xystar_bbox, grid_res)
        for dp in data_points:
            dp.update_uv()

        create_triangle_fans(grid)
        # plot_grid('grid.pdf', data_points, grid, xystar_bbox, False)

        # Compute spectra and store in spectrum_data.h
        compute_spectra(data_points)
        write_output(data_points, grid, grid_res, 
            #'spectra_{}_{}.rs'.format(os.path.splitext(args.cmf)[0], args.scale))
            'meng_spectra_tables.rs')

        # Finally, plot all spectra.
        if args.plot:
            plot_spectra(data_points)

